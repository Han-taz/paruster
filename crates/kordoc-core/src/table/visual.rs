//! Table representation and visual crop selection policy; this module does no rendering.

use kordoc_ir::{IrTable, TableClassificationKind, TableRepresentation};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TableVisualPolicy {
    None,
    NonTabular,
    #[default]
    NonTabularAndUncertain,
    All,
}

pub fn wants_crop(kind: TableClassificationKind, policy: TableVisualPolicy) -> bool {
    match policy {
        TableVisualPolicy::None => false,
        TableVisualPolicy::All => true,
        TableVisualPolicy::NonTabular => kind == TableClassificationKind::NonTabularLayout,
        TableVisualPolicy::NonTabularAndUncertain => {
            matches!(
                kind,
                TableClassificationKind::NonTabularLayout | TableClassificationKind::Uncertain
            )
        }
    }
}

/// Chooses a semantic representation without producing Markdown or image output.
pub fn choose_table_representation(table: &IrTable, smart_visual: bool) -> TableRepresentation {
    if smart_visual
        && table.classification.as_ref().is_some_and(|classification| {
            classification.kind == TableClassificationKind::NonTabularLayout
        })
    {
        return TableRepresentation::Visual;
    }
    let merged = table.cells.iter().any(|row| {
        row.iter()
            .any(|cell| cell.col_span > 1 || cell.row_span > 1)
    });
    let structured = super::has_structured_cell_content(table);
    if merged || structured {
        TableRepresentation::Html
    } else {
        TableRepresentation::Gfm
    }
}

#[cfg(test)]
mod tests {
    use super::{TableVisualPolicy, choose_table_representation, wants_crop};
    use kordoc_ir::{IrBlockType, TableClassificationKind, TableRepresentation};
    use serde_json::json;

    fn table(value: serde_json::Value) -> kordoc_ir::IrTable {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn representation_defaults_to_gfm_or_html() {
        let plain = table(json!({"rows":2,"cols":2,"hasHeader":true,"cells":[
            [{"text":"a","colSpan":1,"rowSpan":1},{"text":"b","colSpan":1,"rowSpan":1}],
            [{"text":"c","colSpan":1,"rowSpan":1},{"text":"d","colSpan":1,"rowSpan":1}]
        ]}));
        assert_eq!(
            choose_table_representation(&plain, false),
            TableRepresentation::Gfm
        );
        let merged = table(json!({"rows":1,"cols":2,"hasHeader":true,"cells":[
            [{"text":"a","colSpan":2,"rowSpan":1},{"text":"","colSpan":1,"rowSpan":1}]
        ]}));
        assert_eq!(
            choose_table_representation(&merged, false),
            TableRepresentation::Html
        );
        let mut layout = plain.clone();
        layout.classification = Some(kordoc_ir::TableClassificationSummary {
            kind: TableClassificationKind::NonTabularLayout,
            confidence: 0.8,
            semantic_score: 0.1,
            non_tabular_score: 0.9,
            reasons: vec![kordoc_ir::TableClassificationReason::ExtremeSparsity],
        });
        assert_eq!(
            choose_table_representation(&layout, false),
            TableRepresentation::Gfm
        );
        assert_eq!(
            choose_table_representation(&layout, true),
            TableRepresentation::Visual
        );
        let structured = table(json!({"rows":1,"cols":1,"hasHeader":false,"cells":[[
            {"text":"x","colSpan":1,"rowSpan":1,"blocks":[{"type":"separator"}]}
        ]]}));
        assert_eq!(
            choose_table_representation(&structured, false),
            TableRepresentation::Html
        );
        assert_eq!(
            IrBlockType::Separator,
            structured.cells[0][0].blocks.as_ref().unwrap()[0].kind
        );
    }

    #[test]
    fn visual_policy_selects_only_requested_kinds() {
        use TableClassificationKind::{
            NonTabularLayout as Layout, SemanticTable as Semantic, Uncertain,
        };
        let policies = [
            (TableVisualPolicy::None, [false, false, false]),
            (TableVisualPolicy::All, [true, true, true]),
            (TableVisualPolicy::NonTabular, [false, true, false]),
            (
                TableVisualPolicy::NonTabularAndUncertain,
                [false, true, true],
            ),
        ];
        for (policy, expected) in policies {
            assert_eq!(
                [
                    wants_crop(Semantic, policy),
                    wants_crop(Layout, policy),
                    wants_crop(Uncertain, policy),
                ],
                expected,
            );
        }
    }
}
