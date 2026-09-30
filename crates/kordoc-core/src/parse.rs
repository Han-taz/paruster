#![allow(dead_code)] // Format adapters are wired by the coordinator after this parser seam lands.

use kordoc_ir::{DocumentMetadata, FileType, KordocError, ParsedDocument};
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Once;

use crate::detect::ParseDispatchError;
use crate::limits::validate_input_len;
use kordoc_ir::ParseOptions;

thread_local! {
    static SUPPRESS_PARSER_PANIC_OUTPUT: Cell<bool> = const { Cell::new(false) };
}

static INSTALL_PARSER_PANIC_HOOK: Once = Once::new();

/// Keep parser panic payloads out of host stderr while preserving the host's panic hook for
/// every other thread. The wrapper is installed once because panic hooks are process-global;
/// suppression itself is thread-local and scoped to the guarded parser call.
pub(crate) fn catch_parser_unwind<F, T>(callback: F) -> std::thread::Result<T>
where
    F: FnOnce() -> T,
{
    INSTALL_PARSER_PANIC_HOOK.call_once(|| {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let suppress = SUPPRESS_PARSER_PANIC_OUTPUT.with(Cell::get);
            if !suppress {
                previous_hook(panic_info);
            }
        }));
    });

    SUPPRESS_PARSER_PANIC_OUTPUT.with(|flag| {
        let previous = flag.replace(true);
        let result = catch_unwind(AssertUnwindSafe(callback));
        flag.set(previous);
        result
    })
}

/// Parser interface used by the core-side format adapters.
///
/// Format crates expose parse functions that return `ParsedDocument`; adapters in this crate
/// implement this trait so format crates never need to depend on `kordoc-core`.
#[allow(dead_code)]
pub(crate) trait Parser: Send + Sync {
    fn parse(
        &self,
        bytes: &[u8],
        file_type: FileType,
        options: &ParseOptions,
    ) -> Result<ParsedDocument, KordocError>;

    /// An optional bounded metadata-only path. Formats without a specialized implementation
    /// return `None`; any full-parse fallback is a caller policy and is not performed here.
    fn parse_metadata(
        &self,
        _bytes: &[u8],
        _file_type: FileType,
        _options: &ParseOptions,
    ) -> Result<Option<DocumentMetadata>, KordocError> {
        Ok(None)
    }
}

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct ParserRegistry {
    parsers: Vec<(FileType, Box<dyn Parser>)>,
}

impl ParserRegistry {
    pub(crate) fn built_in() -> Self {
        Self {
            parsers: vec![(FileType::Hwpx, Box::new(crate::hwpx::HwpxParser))],
        }
    }
    pub(crate) fn register<P>(&mut self, file_type: FileType, parser: P) -> Result<(), FileType>
    where
        P: Parser + 'static,
    {
        if self
            .parsers
            .iter()
            .any(|(registered, _)| *registered == file_type)
        {
            return Err(file_type);
        }
        self.parsers.push((file_type, Box::new(parser)));
        Ok(())
    }

    pub(crate) fn parse(
        &self,
        bytes: &[u8],
        file_type: FileType,
        options: &ParseOptions,
    ) -> Result<ParsedDocument, ParseDispatchError> {
        let parser = self.parser(file_type).ok_or_else(|| {
            dispatch_error(
                file_type,
                KordocError::new(
                    kordoc_ir::ErrorCode::UnsupportedFormat,
                    "Parsing is not implemented for this format",
                ),
                options,
            )
        })?;
        match catch_parser_unwind(|| parser.parse(bytes, file_type, options)) {
            Ok(result) => result.map_err(|error| dispatch_error(file_type, error, options)),
            Err(_) => Err(parser_panic_error(file_type)),
        }
    }

    pub(crate) fn parse_metadata(
        &self,
        bytes: &[u8],
        file_type: FileType,
        options: &ParseOptions,
    ) -> Result<Option<DocumentMetadata>, ParseDispatchError> {
        let parser = self.parser(file_type).ok_or_else(|| {
            dispatch_error(
                file_type,
                KordocError::new(
                    kordoc_ir::ErrorCode::UnsupportedFormat,
                    "Parsing is not implemented for this format",
                ),
                options,
            )
        })?;
        match catch_parser_unwind(|| parser.parse_metadata(bytes, file_type, options)) {
            Ok(result) => result.map_err(|error| dispatch_error(file_type, error, options)),
            Err(_) => Err(parser_panic_error(file_type)),
        }
    }

    fn parser(&self, file_type: FileType) -> Option<&dyn Parser> {
        self.parsers
            .iter()
            .find(|(registered, _)| *registered == file_type)
            .map(|(_, parser)| parser.as_ref())
    }
}

pub(crate) fn parser_panic_error(file_type: FileType) -> ParseDispatchError {
    ParseDispatchError {
        file_type,
        code: kordoc_ir::ErrorCode::ParseError,
        message: "Parser failed unexpectedly".into(),
    }
}

/// Validate and detect the source before looking up a parser. ZIP detection performs the bounded
/// archive preflight before returning a format, so a malformed archive never reaches a parser.
pub(crate) fn try_parse_with_registry(
    bytes: &[u8],
    registry: &ParserRegistry,
    options: &ParseOptions,
) -> Result<(FileType, ParsedDocument), ParseDispatchError> {
    try_parse_with_registry_at_len(bytes, bytes.len(), registry, options)
}

fn try_parse_with_registry_at_len(
    bytes: &[u8],
    input_len: usize,
    registry: &ParserRegistry,
    options: &ParseOptions,
) -> Result<(FileType, ParsedDocument), ParseDispatchError> {
    if bytes.is_empty() {
        return Err(ParseDispatchError {
            file_type: FileType::Unknown,
            code: kordoc_ir::ErrorCode::EmptyInput,
            message: "빈 버퍼이거나 유효하지 않은 입력입니다.".into(),
        });
    }
    validate_input_len(input_len).map_err(ParseDispatchError::from)?;
    let file_type = crate::detect::detect_format(bytes).map_err(ParseDispatchError::from)?;
    let parsed = registry.parse(bytes, file_type, options)?;
    Ok((file_type, parsed))
}

/// Assemble parser data with the projections produced by the core projection layer.
pub(crate) fn assemble_success(
    file_type: FileType,
    mut parsed: ParsedDocument,
    options: &ParseOptions,
) -> Result<kordoc_ir::ParseSuccess, ParseDispatchError> {
    if options.classify_tables == Some(true) {
        crate::table::classifier::classify_table_tree(&mut parsed.blocks)
            .map_err(|error| dispatch_error(file_type, error, options))?;
    }
    let markdown = crate::blocks_to_markdown(&parsed.blocks)
        .map_err(|error| dispatch_error(file_type, error, options))?;
    let pages = crate::blocks_to_pages(
        &parsed.blocks,
        parsed.page_evidence.as_deref(),
        crate::blocks_to_markdown,
    )
    .map_err(|error| dispatch_error(file_type, error, options))?;
    let page_count = parsed.page_count.or_else(|| {
        parsed
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.page_count)
    });
    if options.images == Some(false) {
        parsed.images = None;
        drop_image_data(&mut parsed.blocks, 0);
    }
    let mut success = kordoc_ir::ParseSuccess {
        file_type,
        page_count,
        is_image_based: parsed.is_image_based,
        success: true,
        markdown,
        blocks: parsed.blocks,
        metadata: parsed.metadata,
        outline: parsed.outline,
        warnings: parsed.warnings,
        images: parsed.images,
        pages,
        page_quality: parsed.page_quality,
        quality_summary: parsed.quality_summary,
    };
    crate::postprocess::apply(&mut success, options)
        .map_err(|error| dispatch_error(file_type, error, options))?;
    Ok(success)
}

fn drop_image_data(blocks: &mut [kordoc_ir::IrBlock], depth: usize) {
    if depth > 64 {
        return;
    }
    for block in blocks {
        block.image_data = None;
        if let Some(children) = &mut block.children {
            drop_image_data(children, depth + 1);
        }
        if let Some(table) = &mut block.table {
            for cell in table.cells.iter_mut().flatten() {
                if let Some(cell_blocks) = &mut cell.blocks {
                    drop_image_data(cell_blocks, depth + 1);
                }
            }
            if let Some(caption_blocks) = &mut table.caption_blocks {
                drop_image_data(caption_blocks, depth + 1);
            }
        }
    }
}

pub(crate) fn dispatch_error(
    file_type: FileType,
    error: KordocError,
    options: &ParseOptions,
) -> ParseDispatchError {
    let message = match options
        .password
        .as_deref()
        .filter(|password| !password.is_empty())
    {
        Some(password) => error.message.replace(password, "[REDACTED]"),
        None => error.message,
    };
    ParseDispatchError {
        file_type,
        code: error.code,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Parser, ParserRegistry, assemble_success, try_parse_with_registry,
        try_parse_with_registry_at_len,
    };
    use kordoc_ir::{
        DocumentMetadata, ErrorCode, FileType, KordocError, OcrOption, PageEvidence, PageMarkdown,
        PageNumber, PageSelection, ParseOptions, ParseResult, ParseWarning, ParsedDocument,
        WarningCode,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct RecordingParser;

    impl Parser for RecordingParser {
        fn parse(
            &self,
            bytes: &[u8],
            file_type: FileType,
            options: &ParseOptions,
        ) -> Result<ParsedDocument, KordocError> {
            assert_eq!(file_type, FileType::Pdf);
            assert_eq!(bytes, b"%PDF payload");
            assert_eq!(
                options.pages,
                Some(PageSelection::Numbers(vec![PageNumber::new(1.5).unwrap()]))
            );
            assert_eq!(options.ocr, Some(OcrOption::Force));
            Ok(ParsedDocument {
                page_count: Some(1),
                ..ParsedDocument::default()
            })
        }
    }

    #[test]
    fn registry_dispatches_to_the_registered_parser_with_detected_type_and_options() {
        let mut registry = ParserRegistry::default();
        registry.register(FileType::Pdf, RecordingParser).unwrap();
        let bytes = b"%PDF payload";
        let file_type = crate::detect::detect_format(bytes).unwrap();
        let options = ParseOptions {
            pages: Some(PageSelection::Numbers(vec![PageNumber::new(1.5).unwrap()])),
            ocr: Some(OcrOption::Force),
            ..ParseOptions::default()
        };

        let parsed = registry.parse(bytes, file_type, &options).unwrap();

        assert_eq!(parsed.page_count, Some(1));
    }

    #[test]
    fn registry_returns_typed_unsupported_error_for_unregistered_type() {
        let error = ParserRegistry::default()
            .parse(b"%PDF payload", FileType::Pdf, &ParseOptions::default())
            .unwrap_err();

        assert_eq!(error.file_type, FileType::Pdf);
        assert_eq!(error.code, ErrorCode::UnsupportedFormat);
    }

    #[test]
    fn registry_rejects_duplicate_registration() {
        let mut registry = ParserRegistry::default();
        registry.register(FileType::Pdf, RecordingParser).unwrap();

        assert!(registry.register(FileType::Pdf, RecordingParser).is_err());
    }

    struct FailingParser;

    impl Parser for FailingParser {
        fn parse(
            &self,
            _bytes: &[u8],
            _file_type: FileType,
            _options: &ParseOptions,
        ) -> Result<ParsedDocument, KordocError> {
            Err(KordocError::new(
                ErrorCode::Encrypted,
                "bad password: secret",
            ))
        }
    }

    #[test]
    fn parser_errors_keep_detected_type_and_redact_passwords() {
        let mut registry = ParserRegistry::default();
        registry.register(FileType::Pdf, FailingParser).unwrap();
        let options = ParseOptions {
            password: Some("secret".into()),
            ..ParseOptions::default()
        };

        let error = registry
            .parse(b"%PDF payload", FileType::Pdf, &options)
            .unwrap_err();

        assert_eq!(error.file_type, FileType::Pdf);
        assert_eq!(error.code, ErrorCode::Encrypted);
        assert!(!error.message.contains("secret"));
    }

    struct MetadataParser;

    impl Parser for MetadataParser {
        fn parse(
            &self,
            _bytes: &[u8],
            _file_type: FileType,
            _options: &ParseOptions,
        ) -> Result<ParsedDocument, KordocError> {
            Ok(ParsedDocument::default())
        }

        fn parse_metadata(
            &self,
            _bytes: &[u8],
            _file_type: FileType,
            _options: &ParseOptions,
        ) -> Result<Option<DocumentMetadata>, KordocError> {
            Ok(Some(DocumentMetadata {
                title: Some("metadata-only".into()),
                ..DocumentMetadata::default()
            }))
        }
    }

    #[test]
    fn metadata_hook_defaults_to_none_and_allows_parser_override() {
        let mut registry = ParserRegistry::default();
        registry.register(FileType::Pdf, RecordingParser).unwrap();
        let options = ParseOptions::default();
        assert_eq!(
            registry
                .parse_metadata(b"%PDF payload", FileType::Pdf, &options)
                .unwrap(),
            None
        );

        let mut registry = ParserRegistry::default();
        registry.register(FileType::Pdf, MetadataParser).unwrap();
        assert_eq!(
            registry
                .parse_metadata(b"%PDF payload", FileType::Pdf, &options)
                .unwrap()
                .unwrap()
                .title
                .as_deref(),
            Some("metadata-only")
        );
    }

    struct PanickingParser;

    impl Parser for PanickingParser {
        fn parse(
            &self,
            _bytes: &[u8],
            _file_type: FileType,
            _options: &ParseOptions,
        ) -> Result<ParsedDocument, KordocError> {
            panic!("parser panic payload includes a secret");
        }

        fn parse_metadata(
            &self,
            _bytes: &[u8],
            _file_type: FileType,
            _options: &ParseOptions,
        ) -> Result<Option<DocumentMetadata>, KordocError> {
            panic!("metadata panic payload includes a secret");
        }
    }

    #[test]
    fn parser_panics_become_sanitized_parse_errors() {
        let mut registry = ParserRegistry::default();
        registry.register(FileType::Pdf, PanickingParser).unwrap();

        let error = registry
            .parse(b"%PDF", FileType::Pdf, &ParseOptions::default())
            .unwrap_err();

        assert_eq!(error.file_type, FileType::Pdf);
        assert_eq!(error.code, ErrorCode::ParseError);
        assert_eq!(error.message, "Parser failed unexpectedly");
    }

    #[test]
    fn parser_panic_payload_is_not_written_to_stderr() {
        const CHILD_ENV: &str = "KORDOC_PANIC_STDERR_TEST_CHILD";
        const TEST_NAME: &str = "parse::tests::parser_panic_payload_is_not_written_to_stderr";

        if std::env::var_os(CHILD_ENV).is_some() {
            let mut registry = ParserRegistry::default();
            registry.register(FileType::Pdf, PanickingParser).unwrap();
            let error = registry
                .parse(b"%PDF", FileType::Pdf, &ParseOptions::default())
                .unwrap_err();
            assert_eq!(error.code, ErrorCode::ParseError);
            return;
        }

        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST_NAME, "--nocapture"])
            .env(CHILD_ENV, "1")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("parser panic payload includes a secret"));
    }

    #[test]
    fn metadata_panics_become_sanitized_parse_errors() {
        let mut registry = ParserRegistry::default();
        registry.register(FileType::Pdf, PanickingParser).unwrap();

        let error = registry
            .parse_metadata(b"%PDF", FileType::Pdf, &ParseOptions::default())
            .unwrap_err();

        assert_eq!(error.file_type, FileType::Pdf);
        assert_eq!(error.code, ErrorCode::ParseError);
        assert_eq!(error.message, "Parser failed unexpectedly");
    }

    struct CountingParser(Arc<AtomicUsize>);

    impl Parser for CountingParser {
        fn parse(
            &self,
            _bytes: &[u8],
            _file_type: FileType,
            _options: &ParseOptions,
        ) -> Result<ParsedDocument, KordocError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(ParsedDocument::default())
        }
    }

    #[test]
    fn empty_input_and_zip_preflight_fail_before_registry_dispatch() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = ParserRegistry::default();
        registry
            .register(FileType::Unknown, CountingParser(Arc::clone(&calls)))
            .unwrap();

        let empty = try_parse_with_registry(b"", &registry, &ParseOptions::default()).unwrap_err();
        assert_eq!(empty.code, ErrorCode::EmptyInput);
        let malformed_zip =
            try_parse_with_registry(b"PK\x03\x04truncated", &registry, &ParseOptions::default())
                .unwrap_err();
        assert_eq!(malformed_zip.code, ErrorCode::ZipBomb);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn over_limit_input_fails_before_registry_dispatch() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = ParserRegistry::default();
        registry
            .register(FileType::Unknown, CountingParser(Arc::clone(&calls)))
            .unwrap();
        let error = try_parse_with_registry_at_len(
            b"x",
            crate::limits::MAX_INPUT_BYTES + 1,
            &registry,
            &ParseOptions::default(),
        )
        .unwrap_err();

        assert_eq!(error.code, ErrorCode::OutputTooLarge);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parsed_document_assembly_preserves_recursive_success_fields() {
        let document = ParsedDocument {
            blocks: vec![
                kordoc_ir::IrBlock {
                    page_number: Some(1),
                    children: Some(vec![kordoc_ir::IrBlock::paragraph("nested child")]),
                    ..kordoc_ir::IrBlock::paragraph("outer text")
                },
                kordoc_ir::IrBlock {
                    kind: kordoc_ir::IrBlockType::Table,
                    table: Some(kordoc_ir::IrTable {
                        rows: 1,
                        cols: 1,
                        cells: vec![vec![kordoc_ir::IrCell {
                            text: "cell".into(),
                            col_span: 1,
                            row_span: 1,
                            blocks: Some(vec![kordoc_ir::IrBlock::paragraph("cell child")]),
                            ..kordoc_ir::IrCell::default()
                        }]],
                        has_header: false,
                        caption_blocks: Some(vec![kordoc_ir::IrBlock::paragraph("caption")]),
                        ..kordoc_ir::IrTable::default()
                    }),
                    ..kordoc_ir::IrBlock::default()
                },
            ],
            page_count: Some(2),
            metadata: Some(DocumentMetadata {
                title: Some("fixture".into()),
                page_count: Some(9),
                ..DocumentMetadata::default()
            }),
            warnings: Some(vec![ParseWarning {
                page: Some(2),
                message: "partial".into(),
                code: WarningCode::PartialParse,
            }]),
            page_evidence: Some(vec![
                PageEvidence { page_number: 1 },
                PageEvidence { page_number: 2 },
            ]),
            ..ParsedDocument::default()
        };
        let success = assemble_success(FileType::Pdf, document, &ParseOptions::default()).unwrap();

        assert_eq!(success.file_type, FileType::Pdf);
        assert!(success.success);
        assert_eq!(success.page_count, Some(2));
        assert_eq!(success.metadata.as_ref().unwrap().page_count, Some(9));
        assert_eq!(success.markdown, "outer text\n\ncell");
        assert_eq!(
            success.pages,
            Some(vec![
                PageMarkdown {
                    page_number: 1,
                    markdown: "outer text\n\ncell".into()
                },
                PageMarkdown {
                    page_number: 2,
                    markdown: "".into()
                },
            ])
        );
        assert_eq!(
            success.warnings.as_ref().unwrap()[0].code,
            WarningCode::PartialParse
        );
        assert_eq!(
            success.blocks[0].children.as_ref().unwrap()[0]
                .text
                .as_deref(),
            Some("nested child")
        );
        let table = success.blocks[1].table.as_ref().unwrap();
        assert_eq!(
            table.cells[0][0].blocks.as_ref().unwrap()[0]
                .text
                .as_deref(),
            Some("cell child")
        );
        assert_eq!(
            table.caption_blocks.as_ref().unwrap()[0].text.as_deref(),
            Some("caption")
        );

        let result = ParseResult::Success(success);
        let wire = serde_json::to_value(&result).unwrap();
        assert_eq!(
            wire.pointer("/blocks/0/children/0/text").unwrap(),
            "nested child"
        );
        assert_eq!(
            wire.pointer("/blocks/1/table/cells/0/0/blocks/0/text")
                .unwrap(),
            "cell child"
        );
        assert_eq!(
            wire.pointer("/blocks/1/table/captionBlocks/0/text")
                .unwrap(),
            "caption"
        );
        let roundtrip: ParseResult = serde_json::from_value(wire).unwrap();
        assert_eq!(roundtrip, result);
    }

    #[test]
    fn images_false_drops_payloads_recursively_after_projections() {
        use kordoc_ir::{ExtractedImage, ImageData, IrBlock, IrBlockType, IrCell, IrTable};

        fn image_block() -> IrBlock {
            IrBlock {
                kind: IrBlockType::Image,
                image_data: Some(ImageData {
                    data: vec![1, 2, 3],
                    mime_type: "image/png".into(),
                    filename: Some("pixel.png".into()),
                }),
                ..IrBlock::default()
            }
        }

        let image = ExtractedImage {
            filename: "pixel.png".into(),
            data: vec![1, 2, 3],
            mime_type: "image/png".into(),
            source: Some("BinData/pixel.png".into()),
        };
        let mut root = IrBlock::paragraph("visible text");
        root.image_data = Some(ImageData {
            data: vec![1, 2, 3],
            mime_type: "image/png".into(),
            filename: Some("pixel.png".into()),
        });
        root.children = Some(vec![image_block()]);
        root.table = Some(IrTable {
            rows: 1,
            cols: 1,
            cells: vec![vec![IrCell {
                blocks: Some(vec![image_block()]),
                ..IrCell::default()
            }]],
            caption_blocks: Some(vec![image_block()]),
            ..IrTable::default()
        });
        let parsed = ParsedDocument {
            blocks: vec![root],
            images: Some(vec![image]),
            ..ParsedDocument::default()
        };
        let projected =
            assemble_success(FileType::Hwpx, parsed.clone(), &ParseOptions::default()).unwrap();
        let without_images = assemble_success(
            FileType::Hwpx,
            parsed,
            &ParseOptions {
                images: Some(false),
                ..ParseOptions::default()
            },
        )
        .unwrap();

        assert_eq!(without_images.markdown, projected.markdown);
        assert_eq!(without_images.pages, projected.pages);
        assert!(without_images.images.is_none());
        let root = &without_images.blocks[0];
        assert!(root.image_data.is_none());
        assert!(root.children.as_ref().unwrap()[0].image_data.is_none());
        let table = root.table.as_ref().unwrap();
        assert!(
            table.cells[0][0].blocks.as_ref().unwrap()[0]
                .image_data
                .is_none()
        );
        assert!(
            table.caption_blocks.as_ref().unwrap()[0]
                .image_data
                .is_none()
        );
    }
}
