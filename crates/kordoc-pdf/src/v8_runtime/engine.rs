use super::PdfJsProbe;
use super::allocator::{AllocationCap, new_v8_allocator};
use super::host;
use kordoc_ir::{ErrorCode, KordocError};
use std::sync::Once;
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

const PDFJS_MAIN: &str = include_str!("../../assets/pdfjs/legacy/build/pdf.mjs");
const PDFJS_WORKER: &str = include_str!("../../assets/pdfjs/legacy/build/pdf.worker.mjs");
const HEAP_LIMIT: usize = 192 * 1024 * 1024;
const EXTERNAL_BUFFER_LIMIT: usize = 128 * 1024 * 1024;

static V8_INIT: Once = Once::new();

struct DeadlineGuard {
    cancel: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl DeadlineGuard {
    fn new(isolate: &v8::Isolate, timeout: Duration) -> Self {
        let handle = isolate.thread_safe_handle();
        let (cancel, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            if receiver.recv_timeout(timeout).is_err() {
                handle.terminate_execution();
            }
        });
        Self {
            cancel,
            thread: Some(thread),
        }
    }
}

impl Drop for DeadlineGuard {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn initialize_v8() {
    V8_INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn parse_error(message: &str) -> KordocError {
    KordocError::new(
        ErrorCode::ParseError,
        message.chars().take(512).collect::<String>(),
    )
}

fn ensure_external_capacity(cap: &AllocationCap, bytes: usize) -> Result<(), KordocError> {
    if cap.can_allocate(bytes) {
        Ok(())
    } else {
        Err(KordocError::new(
            ErrorCode::OutputTooLarge,
            "PDF.js external buffer limit exceeded",
        ))
    }
}

pub(super) fn probe(
    bytes: &[u8],
    max_pages: u32,
    max_output: usize,
) -> Result<PdfJsProbe, KordocError> {
    initialize_v8();
    let external_cap = AllocationCap::new(EXTERNAL_BUFFER_LIMIT);
    let mut isolate = v8::Isolate::new(
        v8::Isolate::create_params()
            .set_max_old_generation_size_in_bytes(HEAP_LIMIT)
            .array_buffer_allocator(new_v8_allocator(&external_cap)),
    );
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let _deadline = DeadlineGuard::new(&isolate, Duration::from_secs(10));
    v8::scope!(let handle_scope, &mut isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    v8::scope_with_context!(let scope, handle_scope, context);

    run_script(scope, host::SHIMS)
        .ok_or_else(|| parse_error("PDF.js host initialization failed"))?;
    let worker = compile_module(scope, PDFJS_WORKER)
        .ok_or_else(|| parse_error("PDF.js worker module compilation failed"))?;
    worker
        .instantiate_module(scope, |_context, _specifier, _assertions, _referrer| None)
        .ok_or_else(|| parse_error("PDF.js worker requested an unsupported import"))?;
    worker
        .evaluate(scope)
        .ok_or_else(|| parse_error("PDF.js worker evaluation failed"))?;
    scope.perform_microtask_checkpoint();
    let worker_namespace = worker
        .get_module_namespace()
        .to_object(scope)
        .ok_or_else(|| parse_error("PDF.js worker namespace unavailable"))?;
    let worker_key = v8::String::new(scope, "WorkerMessageHandler").unwrap();
    let worker_handler = worker_namespace
        .get(scope, worker_key.into())
        .ok_or_else(|| parse_error("PDF.js worker handler unavailable"))?;
    let global = context.global(scope);
    let worker_obj = v8::Object::new(scope);
    let key = v8::String::new(scope, "WorkerMessageHandler").unwrap();
    worker_obj.set(scope, key.into(), worker_handler);
    let key = v8::String::new(scope, "pdfjsWorker").unwrap();
    global.set(scope, key.into(), worker_obj.into());

    let main = compile_module(scope, PDFJS_MAIN)
        .ok_or_else(|| parse_error("PDF.js main module compilation failed"))?;
    main.instantiate_module(scope, |_context, _specifier, _assertions, _referrer| None)
        .ok_or_else(|| parse_error("PDF.js main requested an unsupported import"))?;
    main.evaluate(scope)
        .ok_or_else(|| parse_error("PDF.js main evaluation failed"))?;
    scope.perform_microtask_checkpoint();
    if main.get_status() == v8::ModuleStatus::Errored {
        let message = main
            .get_exception()
            .to_string(scope)
            .map(|v| v.to_rust_string_lossy(scope))
            .unwrap_or_default();
        return Err(parse_error(&format!(
            "PDF.js main module errored: {message}"
        )));
    }
    let namespace = main.get_module_namespace();
    let lib_key = v8::String::new(scope, "pdfjsLib").unwrap();
    context.global(scope).set(scope, lib_key.into(), namespace);
    let export_ok = run_script(
        scope,
        "typeof globalThis.pdfjsLib?.getDocument === 'function'",
    )
    .is_some_and(|value| value.boolean_value(scope));
    if !export_ok {
        return Err(parse_error("PDF.js getDocument export is unavailable"));
    }
    let pdfjs_version = run_script(scope, "String(globalThis.pdfjsLib.version)")
        .and_then(|value| value.to_string(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .ok_or_else(|| parse_error("PDF.js version is unavailable"))?;
    if pdfjs_version != "4.10.38" {
        return Err(parse_error(
            "PDF.js runtime version differs from pinned assets",
        ));
    }

    // V8's custom allocator charges this input and subsequent JS ArrayBuffers.
    // The precheck avoids asking V8 to allocate beyond its external cap.
    ensure_external_capacity(&external_cap, bytes.len())?;
    let array_buffer = v8::ArrayBuffer::new(scope, bytes.len());
    for (slot, byte) in array_buffer.get_backing_store().iter().zip(bytes) {
        slot.set(*byte);
    }
    let input = v8::Uint8Array::new(scope, array_buffer, 0, bytes.len())
        .ok_or_else(|| parse_error("could not create PDF.js input buffer"))?;
    let input_key = v8::String::new(scope, "__pdfInput").unwrap();
    context
        .global(scope)
        .set(scope, input_key.into(), input.into());
    let probe_js = format!(
        r#"
      globalThis.__probeState={{done:false,error:null,result:null}};
      const task=pdfjsLib.getDocument({{data:globalThis.__pdfInput,useWorkerFetch:false,isEvalSupported:false,disableFontFace:true,isOffscreenCanvasSupported:false,isImageDecoderSupported:false}});
      const settled=task.promise.then(async doc=>{{
        if(doc.numPages>{max_pages}) throw new Error('page limit exceeded');
        const pageText=[]; let total=0;
        for(let p=1;p<=doc.numPages;p++){{ const page=await doc.getPage(p); const tc=await page.getTextContent();
          if(tc.items.length>100000) throw new Error('text item limit exceeded');
          let text=''; for(const item of tc.items){{
            const part=typeof item.str==='string'?item.str:'';
            for(let i=0;i<part.length;i++){{
              const c=part.charCodeAt(i);
              if(c<128) total+=1;
              else if(c<2048) total+=2;
              else if(c>=0xD800&&c<=0xDBFF&&i+1<part.length){{
                const next=part.charCodeAt(i+1);
                if(next>=0xDC00&&next<=0xDFFF){{total+=4;i++;}} else total+=3;
              }} else total+=3;
            }}
            if(total>{max_output}) throw new Error('text output limit exceeded');
            text+=part;
          }} pageText.push(text); }}
        __probeState.result={{page_count:doc.numPages,page_text:pageText}};
      }}).catch(e=>{{__probeState.error=String(e&&e.message||e);}}).finally(()=>task.destroy());
      settled.then(()=>{{__probeState.done=true;}},e=>{{__probeState.error=String(e&&e.message||e);__probeState.done=true;}});
    "#
    );
    run_script(scope, &probe_js).ok_or_else(|| parse_error("PDF.js rejected the document"))?;
    for _ in 0..1000 {
        scope.perform_microtask_checkpoint();
        let done = get_bool(scope, "__probeState.done");
        if done {
            break;
        }
    }
    if !get_bool(scope, "__probeState.done") {
        return Err(parse_error("PDF.js probe deadline exceeded"));
    }
    if external_cap.rejected() {
        return Err(KordocError::new(
            ErrorCode::OutputTooLarge,
            "PDF.js external buffer limit exceeded",
        ));
    }
    let err = get_string_property(scope, "__probeState", "error");
    if !err.is_empty() {
        if err.contains("limit exceeded") {
            return Err(KordocError::new(ErrorCode::OutputTooLarge, err));
        }
        return Err(parse_error(&err));
    }
    let Some(json) = run_script(scope, "JSON.stringify(globalThis.__probeState.result)")
        .and_then(|value| value.to_string(scope))
    else {
        return Err(parse_error("PDF.js result serialization failed"));
    };
    if json.utf8_length(scope) > max_output {
        return Err(KordocError::new(
            ErrorCode::OutputTooLarge,
            "PDF.js output exceeds runtime limit",
        ));
    }
    let json = json.to_rust_string_lossy(scope);
    serde_json::from_str(&json).map_err(|_| parse_error("PDF.js returned an invalid result"))
}

fn compile_module<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> Option<v8::Local<'s, v8::Module>> {
    let text = v8::String::new(scope, source)?;
    let resource = v8::String::new(scope, "pdfjs://embedded/module")?;
    let origin = v8::ScriptOrigin::new(
        scope,
        resource.into(),
        0,
        0,
        false,
        0,
        None,
        false,
        false,
        true,
        None,
    );
    let mut compiler_source = v8::script_compiler::Source::new(text, Some(&origin));
    v8::script_compiler::compile_module(scope, &mut compiler_source)
}

fn run_script<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let text = v8::String::new(scope, source)?;
    let script = v8::Script::compile(scope, text, None)?;
    script.run(scope)
}

fn get_bool(scope: &mut v8::PinScope<'_, '_>, key: &str) -> bool {
    let source = format!("globalThis.{key}");
    run_script(scope, &source).is_some_and(|value| value.boolean_value(scope))
}

fn get_string_property(scope: &mut v8::PinScope<'_, '_>, object: &str, property: &str) -> String {
    let source = format!("globalThis.{object}.{property} ?? ''");
    let Some(value) = run_script(scope, &source).and_then(|value| value.to_string(scope)) else {
        return String::new();
    };
    if value.utf8_length(scope) > 512 {
        return "PDF.js error message exceeded diagnostic limit".to_owned();
    }
    value.to_rust_string_lossy(scope)
}

#[cfg(test)]
pub(super) fn test_host_has_no_io_globals() -> bool {
    initialize_v8();
    let mut isolate = v8::Isolate::new(v8::CreateParams::default());
    v8::scope!(let base, &mut isolate);
    let context = v8::Context::new(base, Default::default());
    v8::scope_with_context!(let scope, base, context);
    if run_script(scope, host::SHIMS).is_none() {
        return false;
    }
    let Some(worker) = compile_module(scope, PDFJS_WORKER) else {
        return false;
    };
    if worker.instantiate_module(scope, |_context, _specifier, _assertions, _referrer| None)
        != Some(true)
        || worker.evaluate(scope).is_none()
    {
        return false;
    }
    scope.perform_microtask_checkpoint();
    let Some(worker_obj) = worker.get_module_namespace().to_object(scope) else {
        return false;
    };
    let Some(worker_handler) = worker_obj.get(
        scope,
        v8::String::new(scope, "WorkerMessageHandler")
            .unwrap()
            .into(),
    ) else {
        return false;
    };
    let shim_worker = v8::Object::new(scope);
    shim_worker.set(
        scope,
        v8::String::new(scope, "WorkerMessageHandler")
            .unwrap()
            .into(),
        worker_handler,
    );
    context.global(scope).set(
        scope,
        v8::String::new(scope, "pdfjsWorker").unwrap().into(),
        shim_worker.into(),
    );
    let Some(main) = compile_module(scope, PDFJS_MAIN) else {
        return false;
    };
    if main.instantiate_module(scope, |_context, _specifier, _assertions, _referrer| None)
        != Some(true)
        || main.evaluate(scope).is_none()
    {
        return false;
    }
    scope.perform_microtask_checkpoint();
    let source = v8::String::new(scope, "['process','require','fetch','XMLHttpRequest','WebSocket','Worker','document','window'].every(k=>typeof globalThis[k]==='undefined')").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    script
        .run(scope)
        .is_some_and(|value| value.boolean_value(scope))
}

#[cfg(test)]
pub(super) fn test_deadline_termination() -> Result<(), KordocError> {
    initialize_v8();
    let mut isolate = v8::Isolate::new(v8::CreateParams::default());
    let _deadline = DeadlineGuard::new(&isolate, Duration::from_millis(20));
    v8::scope!(let base, &mut isolate);
    let context = v8::Context::new(base, Default::default());
    v8::scope_with_context!(let scope, base, context);
    let source = v8::String::new(scope, "while(true){}").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    if script.run(scope).is_none() {
        return Err(parse_error("execution deadline exceeded"));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn test_external_buffer_limit() -> Result<(), KordocError> {
    initialize_v8();
    let cap = AllocationCap::new(17);
    {
        let mut isolate = v8::Isolate::new(
            v8::CreateParams::default().array_buffer_allocator(new_v8_allocator(&cap)),
        );
        v8::scope!(let base, &mut isolate);
        let context = v8::Context::new(base, Default::default());
        v8::scope_with_context!(let scope, base, context);
        ensure_external_capacity(&cap, 16)?;
        run_script(scope, "globalThis.first = new ArrayBuffer(16)")
            .ok_or_else(|| parse_error("V8 did not allocate the first test buffer"))?;
        if cap.used() != 16 {
            return Err(parse_error("V8 did not charge the first test buffer"));
        }
        ensure_external_capacity(&cap, 1)?;
        run_script(scope, "globalThis.second = new ArrayBuffer(1)")
            .ok_or_else(|| parse_error("V8 did not allocate the last test byte"))?;
        if cap.used() != 17 {
            return Err(parse_error("V8 did not charge the last test byte"));
        }
        let attempted = run_script(
            scope,
            "try { new ArrayBuffer(1); 'unexpected' } catch (error) { error.name }",
        )
        .and_then(|value| value.to_string(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .ok_or_else(|| parse_error("V8 over-cap allocation did not return a result"))?;
        if attempted != "RangeError" || !cap.rejected() || cap.used() != 17 {
            return Err(parse_error("V8 did not reject the over-cap ArrayBuffer"));
        }
        let rejected = ensure_external_capacity(&cap, 1).unwrap_err();
        if rejected.code != ErrorCode::OutputTooLarge {
            return Err(parse_error("external cap returned the wrong error code"));
        }
    }
    if cap.used() != 0 {
        return Err(parse_error(
            "V8 did not release external buffers with the isolate",
        ));
    }
    Err(KordocError::new(
        ErrorCode::OutputTooLarge,
        "PDF.js external buffer limit exceeded",
    ))
}

#[cfg(test)]
pub(super) fn test_v8_engine_version() -> &'static str {
    v8::V8::get_version()
}
