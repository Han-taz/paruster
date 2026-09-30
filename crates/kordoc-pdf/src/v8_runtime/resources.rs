//! Compile-time allowlist and per-document budgets for PDF.js resource fetches.

use super::allocator::AllocationCap;
use kordoc_ir::{ErrorCode, KordocError};
use std::ffi::c_void;
use std::sync::Arc;

pub(super) const MAX_RESOURCE_ITEM_BYTES: usize = 192 * 1024;
pub(super) const MAX_RESOURCE_REQUESTS: usize = 512;
pub(super) const MAX_RESOURCE_BYTES: usize = 8 * 1024 * 1024;
const MAX_RESOURCE_NAME_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResourceKind {
    CMap,
    StandardFont,
}

impl ResourceKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "cmap" => Some(Self::CMap),
            "standard_font" => Some(Self::StandardFont),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ResourceStats {
    pub(crate) requests: usize,
    pub(crate) cmaps: usize,
    pub(crate) standard_fonts: usize,
    pub(crate) bytes: usize,
    pub(crate) cmap_bytes: usize,
    pub(crate) standard_font_bytes: usize,
    pub(crate) max_item_bytes: usize,
    pub(crate) denied: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ResourceLimits {
    pub(super) max_item_bytes: usize,
    pub(super) max_requests: usize,
    pub(super) max_total_bytes: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_item_bytes: MAX_RESOURCE_ITEM_BYTES,
            max_requests: MAX_RESOURCE_REQUESTS,
            max_total_bytes: MAX_RESOURCE_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResourceFailure {
    Unsupported,
    LimitExceeded,
}

impl ResourceFailure {
    pub(super) fn to_error(self) -> KordocError {
        match self {
            Self::Unsupported => KordocError::new(
                ErrorCode::ParseError,
                "PDF references an unsupported embedded resource",
            ),
            Self::LimitExceeded => KordocError::new(
                ErrorCode::OutputTooLarge,
                "PDF.js embedded resource limit exceeded",
            ),
        }
    }
}

pub(super) struct ResourceBudget {
    limits: ResourceLimits,
    stats: ResourceStats,
    reserved_bytes: usize,
    failure: Option<ResourceFailure>,
}

pub(super) struct ResourceState {
    budget: ResourceBudget,
    allocation_cap: Arc<AllocationCap>,
}

impl ResourceState {
    pub(super) fn new(limits: ResourceLimits, allocation_cap: Arc<AllocationCap>) -> Self {
        Self {
            budget: ResourceBudget::new(limits),
            allocation_cap,
        }
    }

    pub(super) fn failure(&self) -> Option<ResourceFailure> {
        self.budget.failure()
    }

    pub(super) fn stats(&self) -> ResourceStats {
        self.budget.stats()
    }
}

pub(super) fn install_callback(
    scope: &mut v8::PinScope<'_, '_>,
    global: v8::Local<v8::Object>,
    state: &mut ResourceState,
) -> bool {
    let state_pointer = (state as *mut ResourceState).cast::<c_void>();
    let external = v8::External::new(scope, state_pointer);
    let Some(callback) = v8::Function::builder(fetch_resource_callback)
        .data(external.into())
        .build(scope)
    else {
        return false;
    };
    let Some(key) = v8::String::new(scope, "__pdfjsReadEmbeddedResource") else {
        return false;
    };
    global
        .set(scope, key.into(), callback.into())
        .unwrap_or(false)
}

fn fetch_resource_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut return_value: v8::ReturnValue<v8::Value>,
) {
    let Some(external) = v8::Local::<v8::External>::try_from(args.data()).ok() else {
        throw_resource_error(scope, ResourceFailure::Unsupported);
        return;
    };
    let state_pointer = external.value().cast::<ResourceState>();
    if state_pointer.is_null() {
        throw_resource_error(scope, ResourceFailure::Unsupported);
        return;
    }
    // SAFETY: install_callback stores a pointer to ResourceState, which the
    // caller keeps alive until after the V8 isolate and its callbacks are dropped.
    let state = unsafe { &mut *state_pointer };
    if let Err(failure) = state.budget.begin_request() {
        throw_resource_error(scope, failure);
        return;
    }
    let Some(kind) = bounded_ascii_string(scope, args.get(0), 16) else {
        state.budget.record_denied(ResourceFailure::Unsupported);
        throw_resource_error(scope, ResourceFailure::Unsupported);
        return;
    };
    let Some(name) = bounded_ascii_string(scope, args.get(1), MAX_RESOURCE_NAME_BYTES) else {
        state.budget.record_denied(ResourceFailure::Unsupported);
        throw_resource_error(scope, ResourceFailure::Unsupported);
        return;
    };
    let (kind, bytes) = match state.budget.authorize_reserved(&kind, &name) {
        Ok(resource) => resource,
        Err(failure) => {
            throw_resource_error(scope, failure);
            return;
        }
    };
    if !state.allocation_cap.can_allocate(bytes.len()) {
        state.budget.record_denied(ResourceFailure::LimitExceeded);
        throw_resource_error(scope, ResourceFailure::LimitExceeded);
        return;
    }
    let buffer = v8::ArrayBuffer::new(scope, bytes.len());
    for (slot, byte) in buffer.get_backing_store().iter().zip(bytes) {
        slot.set(*byte);
    }
    let Some(view) = v8::Uint8Array::new(scope, buffer, 0, bytes.len()) else {
        state.budget.record_denied(ResourceFailure::LimitExceeded);
        throw_resource_error(scope, ResourceFailure::LimitExceeded);
        return;
    };
    state.budget.record_success(kind, bytes.len());
    return_value.set(view.into());
}

fn bounded_ascii_string(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
    max_bytes: usize,
) -> Option<String> {
    let value = v8::Local::<v8::String>::try_from(value).ok()?;
    if value.length() > max_bytes || value.utf8_length(scope) > max_bytes {
        return None;
    }
    let string = value.to_rust_string_lossy(scope);
    string.is_ascii().then_some(string)
}

fn throw_resource_error(scope: &mut v8::PinScope, failure: ResourceFailure) {
    let message = match failure {
        ResourceFailure::Unsupported => "unsupported PDF.js embedded resource request",
        ResourceFailure::LimitExceeded => "PDF.js embedded resource limit exceeded",
    };
    if let Some(message) = v8::String::new(scope, message) {
        let exception = v8::Exception::type_error(scope, message);
        scope.throw_exception(exception);
    }
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self::new(ResourceLimits::default())
    }
}

impl ResourceBudget {
    pub(super) fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            stats: ResourceStats::default(),
            reserved_bytes: 0,
            failure: None,
        }
    }

    pub(super) fn authorize(
        &mut self,
        kind: &str,
        name: &str,
    ) -> Result<(ResourceKind, &'static [u8]), ResourceFailure> {
        self.begin_request()?;
        self.authorize_reserved(kind, name)
    }

    pub(super) fn begin_request(&mut self) -> Result<(), ResourceFailure> {
        self.stats.requests = self.stats.requests.saturating_add(1);
        if self.stats.requests > self.limits.max_requests {
            return self.reject(ResourceFailure::LimitExceeded);
        }
        Ok(())
    }

    fn authorize_reserved(
        &mut self,
        kind: &str,
        name: &str,
    ) -> Result<(ResourceKind, &'static [u8]), ResourceFailure> {
        let Some(kind) = ResourceKind::parse(kind) else {
            return self.reject(ResourceFailure::Unsupported);
        };
        if name.is_empty()
            || name.len() > MAX_RESOURCE_NAME_BYTES
            || !name.is_ascii()
            || name
                .bytes()
                .any(|byte| !byte.is_ascii_alphanumeric() && !b"._-".contains(&byte))
        {
            return self.reject(ResourceFailure::Unsupported);
        }
        let Some(bytes) = resource_bytes(kind, name) else {
            return self.reject(ResourceFailure::Unsupported);
        };
        if bytes.len() > self.limits.max_item_bytes {
            return self.reject(ResourceFailure::LimitExceeded);
        }
        let Some(total) = self.reserved_bytes.checked_add(bytes.len()) else {
            return self.reject(ResourceFailure::LimitExceeded);
        };
        if total > self.limits.max_total_bytes {
            return self.reject(ResourceFailure::LimitExceeded);
        }
        self.reserved_bytes = total;
        Ok((kind, bytes))
    }

    pub(super) fn record_success(&mut self, kind: ResourceKind, bytes: usize) {
        match kind {
            ResourceKind::CMap => {
                self.stats.cmaps += 1;
                self.stats.cmap_bytes += bytes;
            }
            ResourceKind::StandardFont => {
                self.stats.standard_fonts += 1;
                self.stats.standard_font_bytes += bytes;
            }
        }
        self.stats.bytes += bytes;
        self.stats.max_item_bytes = self.stats.max_item_bytes.max(bytes);
    }

    pub(super) fn record_denied(&mut self, failure: ResourceFailure) {
        self.stats.denied = self.stats.denied.saturating_add(1);
        self.failure.get_or_insert(failure);
    }

    pub(super) fn failure(&self) -> Option<ResourceFailure> {
        self.failure
    }

    pub(super) fn stats(&self) -> ResourceStats {
        self.stats
    }

    fn reject<T>(&mut self, failure: ResourceFailure) -> Result<T, ResourceFailure> {
        self.record_denied(failure);
        Err(failure)
    }
}

fn resource_bytes(kind: ResourceKind, name: &str) -> Option<&'static [u8]> {
    let table = match kind {
        ResourceKind::CMap => CMAPS,
        ResourceKind::StandardFont => STANDARD_FONTS,
    };
    table
        .binary_search_by_key(&name, |(entry_name, _)| *entry_name)
        .ok()
        .map(|index| table[index].1)
}

const CMAPS: &[(&str, &[u8])] = &[
    // Entries are the pinned PDF.js 4.10.38 .bcmap allowlist.
    (
        "78-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/78-EUC-H.bcmap"),
    ),
    (
        "78-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/78-EUC-V.bcmap"),
    ),
    (
        "78-H",
        include_bytes!("../../assets/pdfjs/cmaps/78-H.bcmap"),
    ),
    (
        "78-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/78-RKSJ-H.bcmap"),
    ),
    (
        "78-RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/78-RKSJ-V.bcmap"),
    ),
    (
        "78-V",
        include_bytes!("../../assets/pdfjs/cmaps/78-V.bcmap"),
    ),
    (
        "78ms-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/78ms-RKSJ-H.bcmap"),
    ),
    (
        "78ms-RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/78ms-RKSJ-V.bcmap"),
    ),
    (
        "83pv-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/83pv-RKSJ-H.bcmap"),
    ),
    (
        "90ms-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/90ms-RKSJ-H.bcmap"),
    ),
    (
        "90ms-RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/90ms-RKSJ-V.bcmap"),
    ),
    (
        "90msp-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/90msp-RKSJ-H.bcmap"),
    ),
    (
        "90msp-RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/90msp-RKSJ-V.bcmap"),
    ),
    (
        "90pv-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/90pv-RKSJ-H.bcmap"),
    ),
    (
        "90pv-RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/90pv-RKSJ-V.bcmap"),
    ),
    (
        "Add-H",
        include_bytes!("../../assets/pdfjs/cmaps/Add-H.bcmap"),
    ),
    (
        "Add-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/Add-RKSJ-H.bcmap"),
    ),
    (
        "Add-RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/Add-RKSJ-V.bcmap"),
    ),
    (
        "Add-V",
        include_bytes!("../../assets/pdfjs/cmaps/Add-V.bcmap"),
    ),
    (
        "Adobe-CNS1-0",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-0.bcmap"),
    ),
    (
        "Adobe-CNS1-1",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-1.bcmap"),
    ),
    (
        "Adobe-CNS1-2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-2.bcmap"),
    ),
    (
        "Adobe-CNS1-3",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-3.bcmap"),
    ),
    (
        "Adobe-CNS1-4",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-4.bcmap"),
    ),
    (
        "Adobe-CNS1-5",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-5.bcmap"),
    ),
    (
        "Adobe-CNS1-6",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-6.bcmap"),
    ),
    (
        "Adobe-CNS1-UCS2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-CNS1-UCS2.bcmap"),
    ),
    (
        "Adobe-GB1-0",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-GB1-0.bcmap"),
    ),
    (
        "Adobe-GB1-1",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-GB1-1.bcmap"),
    ),
    (
        "Adobe-GB1-2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-GB1-2.bcmap"),
    ),
    (
        "Adobe-GB1-3",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-GB1-3.bcmap"),
    ),
    (
        "Adobe-GB1-4",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-GB1-4.bcmap"),
    ),
    (
        "Adobe-GB1-5",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-GB1-5.bcmap"),
    ),
    (
        "Adobe-GB1-UCS2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-GB1-UCS2.bcmap"),
    ),
    (
        "Adobe-Japan1-0",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-0.bcmap"),
    ),
    (
        "Adobe-Japan1-1",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-1.bcmap"),
    ),
    (
        "Adobe-Japan1-2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-2.bcmap"),
    ),
    (
        "Adobe-Japan1-3",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-3.bcmap"),
    ),
    (
        "Adobe-Japan1-4",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-4.bcmap"),
    ),
    (
        "Adobe-Japan1-5",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-5.bcmap"),
    ),
    (
        "Adobe-Japan1-6",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-6.bcmap"),
    ),
    (
        "Adobe-Japan1-UCS2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Japan1-UCS2.bcmap"),
    ),
    (
        "Adobe-Korea1-0",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Korea1-0.bcmap"),
    ),
    (
        "Adobe-Korea1-1",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Korea1-1.bcmap"),
    ),
    (
        "Adobe-Korea1-2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Korea1-2.bcmap"),
    ),
    (
        "Adobe-Korea1-UCS2",
        include_bytes!("../../assets/pdfjs/cmaps/Adobe-Korea1-UCS2.bcmap"),
    ),
    (
        "B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/B5-H.bcmap"),
    ),
    (
        "B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/B5-V.bcmap"),
    ),
    (
        "B5pc-H",
        include_bytes!("../../assets/pdfjs/cmaps/B5pc-H.bcmap"),
    ),
    (
        "B5pc-V",
        include_bytes!("../../assets/pdfjs/cmaps/B5pc-V.bcmap"),
    ),
    (
        "CNS-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/CNS-EUC-H.bcmap"),
    ),
    (
        "CNS-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/CNS-EUC-V.bcmap"),
    ),
    (
        "CNS1-H",
        include_bytes!("../../assets/pdfjs/cmaps/CNS1-H.bcmap"),
    ),
    (
        "CNS1-V",
        include_bytes!("../../assets/pdfjs/cmaps/CNS1-V.bcmap"),
    ),
    (
        "CNS2-H",
        include_bytes!("../../assets/pdfjs/cmaps/CNS2-H.bcmap"),
    ),
    (
        "CNS2-V",
        include_bytes!("../../assets/pdfjs/cmaps/CNS2-V.bcmap"),
    ),
    (
        "ETHK-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/ETHK-B5-H.bcmap"),
    ),
    (
        "ETHK-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/ETHK-B5-V.bcmap"),
    ),
    (
        "ETen-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/ETen-B5-H.bcmap"),
    ),
    (
        "ETen-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/ETen-B5-V.bcmap"),
    ),
    (
        "ETenms-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/ETenms-B5-H.bcmap"),
    ),
    (
        "ETenms-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/ETenms-B5-V.bcmap"),
    ),
    (
        "EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/EUC-H.bcmap"),
    ),
    (
        "EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/EUC-V.bcmap"),
    ),
    (
        "Ext-H",
        include_bytes!("../../assets/pdfjs/cmaps/Ext-H.bcmap"),
    ),
    (
        "Ext-RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/Ext-RKSJ-H.bcmap"),
    ),
    (
        "Ext-RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/Ext-RKSJ-V.bcmap"),
    ),
    (
        "Ext-V",
        include_bytes!("../../assets/pdfjs/cmaps/Ext-V.bcmap"),
    ),
    (
        "GB-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/GB-EUC-H.bcmap"),
    ),
    (
        "GB-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/GB-EUC-V.bcmap"),
    ),
    (
        "GB-H",
        include_bytes!("../../assets/pdfjs/cmaps/GB-H.bcmap"),
    ),
    (
        "GB-V",
        include_bytes!("../../assets/pdfjs/cmaps/GB-V.bcmap"),
    ),
    (
        "GBK-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/GBK-EUC-H.bcmap"),
    ),
    (
        "GBK-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/GBK-EUC-V.bcmap"),
    ),
    (
        "GBK2K-H",
        include_bytes!("../../assets/pdfjs/cmaps/GBK2K-H.bcmap"),
    ),
    (
        "GBK2K-V",
        include_bytes!("../../assets/pdfjs/cmaps/GBK2K-V.bcmap"),
    ),
    (
        "GBKp-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/GBKp-EUC-H.bcmap"),
    ),
    (
        "GBKp-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/GBKp-EUC-V.bcmap"),
    ),
    (
        "GBT-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/GBT-EUC-H.bcmap"),
    ),
    (
        "GBT-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/GBT-EUC-V.bcmap"),
    ),
    (
        "GBT-H",
        include_bytes!("../../assets/pdfjs/cmaps/GBT-H.bcmap"),
    ),
    (
        "GBT-V",
        include_bytes!("../../assets/pdfjs/cmaps/GBT-V.bcmap"),
    ),
    (
        "GBTpc-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/GBTpc-EUC-H.bcmap"),
    ),
    (
        "GBTpc-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/GBTpc-EUC-V.bcmap"),
    ),
    (
        "GBpc-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/GBpc-EUC-H.bcmap"),
    ),
    (
        "GBpc-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/GBpc-EUC-V.bcmap"),
    ),
    ("H", include_bytes!("../../assets/pdfjs/cmaps/H.bcmap")),
    (
        "HKdla-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/HKdla-B5-H.bcmap"),
    ),
    (
        "HKdla-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/HKdla-B5-V.bcmap"),
    ),
    (
        "HKdlb-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/HKdlb-B5-H.bcmap"),
    ),
    (
        "HKdlb-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/HKdlb-B5-V.bcmap"),
    ),
    (
        "HKgccs-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/HKgccs-B5-H.bcmap"),
    ),
    (
        "HKgccs-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/HKgccs-B5-V.bcmap"),
    ),
    (
        "HKm314-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/HKm314-B5-H.bcmap"),
    ),
    (
        "HKm314-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/HKm314-B5-V.bcmap"),
    ),
    (
        "HKm471-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/HKm471-B5-H.bcmap"),
    ),
    (
        "HKm471-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/HKm471-B5-V.bcmap"),
    ),
    (
        "HKscs-B5-H",
        include_bytes!("../../assets/pdfjs/cmaps/HKscs-B5-H.bcmap"),
    ),
    (
        "HKscs-B5-V",
        include_bytes!("../../assets/pdfjs/cmaps/HKscs-B5-V.bcmap"),
    ),
    (
        "Hankaku",
        include_bytes!("../../assets/pdfjs/cmaps/Hankaku.bcmap"),
    ),
    (
        "Hiragana",
        include_bytes!("../../assets/pdfjs/cmaps/Hiragana.bcmap"),
    ),
    (
        "KSC-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/KSC-EUC-H.bcmap"),
    ),
    (
        "KSC-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/KSC-EUC-V.bcmap"),
    ),
    (
        "KSC-H",
        include_bytes!("../../assets/pdfjs/cmaps/KSC-H.bcmap"),
    ),
    (
        "KSC-Johab-H",
        include_bytes!("../../assets/pdfjs/cmaps/KSC-Johab-H.bcmap"),
    ),
    (
        "KSC-Johab-V",
        include_bytes!("../../assets/pdfjs/cmaps/KSC-Johab-V.bcmap"),
    ),
    (
        "KSC-V",
        include_bytes!("../../assets/pdfjs/cmaps/KSC-V.bcmap"),
    ),
    (
        "KSCms-UHC-H",
        include_bytes!("../../assets/pdfjs/cmaps/KSCms-UHC-H.bcmap"),
    ),
    (
        "KSCms-UHC-HW-H",
        include_bytes!("../../assets/pdfjs/cmaps/KSCms-UHC-HW-H.bcmap"),
    ),
    (
        "KSCms-UHC-HW-V",
        include_bytes!("../../assets/pdfjs/cmaps/KSCms-UHC-HW-V.bcmap"),
    ),
    (
        "KSCms-UHC-V",
        include_bytes!("../../assets/pdfjs/cmaps/KSCms-UHC-V.bcmap"),
    ),
    (
        "KSCpc-EUC-H",
        include_bytes!("../../assets/pdfjs/cmaps/KSCpc-EUC-H.bcmap"),
    ),
    (
        "KSCpc-EUC-V",
        include_bytes!("../../assets/pdfjs/cmaps/KSCpc-EUC-V.bcmap"),
    ),
    (
        "Katakana",
        include_bytes!("../../assets/pdfjs/cmaps/Katakana.bcmap"),
    ),
    (
        "NWP-H",
        include_bytes!("../../assets/pdfjs/cmaps/NWP-H.bcmap"),
    ),
    (
        "NWP-V",
        include_bytes!("../../assets/pdfjs/cmaps/NWP-V.bcmap"),
    ),
    (
        "RKSJ-H",
        include_bytes!("../../assets/pdfjs/cmaps/RKSJ-H.bcmap"),
    ),
    (
        "RKSJ-V",
        include_bytes!("../../assets/pdfjs/cmaps/RKSJ-V.bcmap"),
    ),
    (
        "Roman",
        include_bytes!("../../assets/pdfjs/cmaps/Roman.bcmap"),
    ),
    (
        "UniCNS-UCS2-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UCS2-H.bcmap"),
    ),
    (
        "UniCNS-UCS2-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UCS2-V.bcmap"),
    ),
    (
        "UniCNS-UTF16-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UTF16-H.bcmap"),
    ),
    (
        "UniCNS-UTF16-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UTF16-V.bcmap"),
    ),
    (
        "UniCNS-UTF32-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UTF32-H.bcmap"),
    ),
    (
        "UniCNS-UTF32-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UTF32-V.bcmap"),
    ),
    (
        "UniCNS-UTF8-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UTF8-H.bcmap"),
    ),
    (
        "UniCNS-UTF8-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniCNS-UTF8-V.bcmap"),
    ),
    (
        "UniGB-UCS2-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UCS2-H.bcmap"),
    ),
    (
        "UniGB-UCS2-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UCS2-V.bcmap"),
    ),
    (
        "UniGB-UTF16-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UTF16-H.bcmap"),
    ),
    (
        "UniGB-UTF16-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UTF16-V.bcmap"),
    ),
    (
        "UniGB-UTF32-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UTF32-H.bcmap"),
    ),
    (
        "UniGB-UTF32-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UTF32-V.bcmap"),
    ),
    (
        "UniGB-UTF8-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UTF8-H.bcmap"),
    ),
    (
        "UniGB-UTF8-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniGB-UTF8-V.bcmap"),
    ),
    (
        "UniJIS-UCS2-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UCS2-H.bcmap"),
    ),
    (
        "UniJIS-UCS2-HW-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UCS2-HW-H.bcmap"),
    ),
    (
        "UniJIS-UCS2-HW-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UCS2-HW-V.bcmap"),
    ),
    (
        "UniJIS-UCS2-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UCS2-V.bcmap"),
    ),
    (
        "UniJIS-UTF16-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UTF16-H.bcmap"),
    ),
    (
        "UniJIS-UTF16-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UTF16-V.bcmap"),
    ),
    (
        "UniJIS-UTF32-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UTF32-H.bcmap"),
    ),
    (
        "UniJIS-UTF32-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UTF32-V.bcmap"),
    ),
    (
        "UniJIS-UTF8-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UTF8-H.bcmap"),
    ),
    (
        "UniJIS-UTF8-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS-UTF8-V.bcmap"),
    ),
    (
        "UniJIS2004-UTF16-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS2004-UTF16-H.bcmap"),
    ),
    (
        "UniJIS2004-UTF16-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS2004-UTF16-V.bcmap"),
    ),
    (
        "UniJIS2004-UTF32-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS2004-UTF32-H.bcmap"),
    ),
    (
        "UniJIS2004-UTF32-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS2004-UTF32-V.bcmap"),
    ),
    (
        "UniJIS2004-UTF8-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS2004-UTF8-H.bcmap"),
    ),
    (
        "UniJIS2004-UTF8-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJIS2004-UTF8-V.bcmap"),
    ),
    (
        "UniJISPro-UCS2-HW-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJISPro-UCS2-HW-V.bcmap"),
    ),
    (
        "UniJISPro-UCS2-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJISPro-UCS2-V.bcmap"),
    ),
    (
        "UniJISPro-UTF8-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJISPro-UTF8-V.bcmap"),
    ),
    (
        "UniJISX0213-UTF32-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJISX0213-UTF32-H.bcmap"),
    ),
    (
        "UniJISX0213-UTF32-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJISX0213-UTF32-V.bcmap"),
    ),
    (
        "UniJISX02132004-UTF32-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniJISX02132004-UTF32-H.bcmap"),
    ),
    (
        "UniJISX02132004-UTF32-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniJISX02132004-UTF32-V.bcmap"),
    ),
    (
        "UniKS-UCS2-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UCS2-H.bcmap"),
    ),
    (
        "UniKS-UCS2-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UCS2-V.bcmap"),
    ),
    (
        "UniKS-UTF16-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UTF16-H.bcmap"),
    ),
    (
        "UniKS-UTF16-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UTF16-V.bcmap"),
    ),
    (
        "UniKS-UTF32-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UTF32-H.bcmap"),
    ),
    (
        "UniKS-UTF32-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UTF32-V.bcmap"),
    ),
    (
        "UniKS-UTF8-H",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UTF8-H.bcmap"),
    ),
    (
        "UniKS-UTF8-V",
        include_bytes!("../../assets/pdfjs/cmaps/UniKS-UTF8-V.bcmap"),
    ),
    ("V", include_bytes!("../../assets/pdfjs/cmaps/V.bcmap")),
    (
        "WP-Symbol",
        include_bytes!("../../assets/pdfjs/cmaps/WP-Symbol.bcmap"),
    ),
];

const STANDARD_FONTS: &[(&str, &[u8])] = &[
    // Entries match PDF.js 4.10.38's standardFontNameToFileName table.
    (
        "FoxitDingbats.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitDingbats.pfb"),
    ),
    (
        "FoxitFixed.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitFixed.pfb"),
    ),
    (
        "FoxitFixedBold.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitFixedBold.pfb"),
    ),
    (
        "FoxitFixedBoldItalic.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitFixedBoldItalic.pfb"),
    ),
    (
        "FoxitFixedItalic.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitFixedItalic.pfb"),
    ),
    (
        "FoxitSerif.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitSerif.pfb"),
    ),
    (
        "FoxitSerifBold.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitSerifBold.pfb"),
    ),
    (
        "FoxitSerifBoldItalic.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitSerifBoldItalic.pfb"),
    ),
    (
        "FoxitSerifItalic.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitSerifItalic.pfb"),
    ),
    (
        "FoxitSymbol.pfb",
        include_bytes!("../../assets/pdfjs/standard_fonts/FoxitSymbol.pfb"),
    ),
    (
        "LiberationSans-Bold.ttf",
        include_bytes!("../../assets/pdfjs/standard_fonts/LiberationSans-Bold.ttf"),
    ),
    (
        "LiberationSans-BoldItalic.ttf",
        include_bytes!("../../assets/pdfjs/standard_fonts/LiberationSans-BoldItalic.ttf"),
    ),
    (
        "LiberationSans-Italic.ttf",
        include_bytes!("../../assets/pdfjs/standard_fonts/LiberationSans-Italic.ttf"),
    ),
    (
        "LiberationSans-Regular.ttf",
        include_bytes!("../../assets/pdfjs/standard_fonts/LiberationSans-Regular.ttf"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;

    #[test]
    fn exact_asset_inventories_are_embedded_and_bounded() {
        assert_eq!(CMAPS.len(), 168);
        assert_eq!(STANDARD_FONTS.len(), 14);
        assert!(CMAPS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(STANDARD_FONTS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(
            CMAPS
                .iter()
                .chain(STANDARD_FONTS.iter())
                .all(|(_, bytes)| bytes.len() <= MAX_RESOURCE_ITEM_BYTES)
        );
    }

    #[test]
    fn embedded_allowlist_bytes_match_the_pinned_asset_manifest() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../assets/pdfjs/PROVENANCE.json"))
                .expect("parse PDF.js provenance manifest");
        let entries = manifest["selected_files"]
            .as_array()
            .expect("selected asset entries");
        assert_eq!(entries.len(), 188);
        for (prefix, table, extension) in [
            ("cmaps/", CMAPS, ".bcmap"),
            ("standard_fonts/", STANDARD_FONTS, ""),
        ] {
            for (name, bytes) in table {
                let path = format!("{prefix}{name}{extension}");
                let entry = entries
                    .iter()
                    .find(|entry| entry["path"] == path)
                    .unwrap_or_else(|| panic!("missing manifest entry for {path}"));
                assert_eq!(entry["bytes"].as_u64(), Some(bytes.len() as u64), "{path}");
                let expected_hash = hex(&sha2::Sha256::digest(bytes));
                assert_eq!(
                    entry["sha256"].as_str(),
                    Some(expected_hash.as_str()),
                    "{path}"
                );
            }
        }
    }

    fn hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
        encoded
    }

    #[test]
    fn resource_names_are_exact_basenames_and_unknown_names_fail() {
        let mut budget = ResourceBudget::default();
        assert!(budget.authorize("cmap", "UniKS-UCS2-H").is_ok());
        for invalid in ["../UniKS-UCS2-H", "https://example.invalid/x", "not-a-cmap"] {
            assert_eq!(
                budget.authorize("cmap", invalid).unwrap_err(),
                ResourceFailure::Unsupported
            );
        }
        assert_eq!(
            budget
                .authorize(
                    "file",
                    "crates/kordoc-pdf/assets/pdfjs/cmaps/UniKS-UCS2-H.bcmap"
                )
                .unwrap_err(),
            ResourceFailure::Unsupported
        );
        assert_eq!(budget.stats().denied, 4);
    }

    #[test]
    fn resource_budget_limits_are_inclusive() {
        let bytes = resource_bytes(ResourceKind::CMap, "UniKS-UCS2-H").unwrap();
        let mut inclusive = ResourceBudget::new(ResourceLimits {
            max_item_bytes: bytes.len(),
            max_requests: 1,
            max_total_bytes: bytes.len(),
        });
        let (kind, authorized) = inclusive.authorize("cmap", "UniKS-UCS2-H").unwrap();
        assert_eq!(authorized.len(), bytes.len());
        inclusive.record_success(kind, authorized.len());
        assert_eq!(inclusive.stats().bytes, bytes.len());

        let mut item_overflow = ResourceBudget::new(ResourceLimits {
            max_item_bytes: bytes.len() - 1,
            max_requests: 1,
            max_total_bytes: bytes.len(),
        });
        assert_eq!(
            item_overflow.authorize("cmap", "UniKS-UCS2-H").unwrap_err(),
            ResourceFailure::LimitExceeded
        );

        let mut total_overflow = ResourceBudget::new(ResourceLimits {
            max_item_bytes: bytes.len(),
            max_requests: 1,
            max_total_bytes: bytes.len() - 1,
        });
        assert_eq!(
            total_overflow
                .authorize("cmap", "UniKS-UCS2-H")
                .unwrap_err(),
            ResourceFailure::LimitExceeded
        );

        let mut request_overflow = ResourceBudget::new(ResourceLimits {
            max_item_bytes: bytes.len(),
            max_requests: 1,
            max_total_bytes: bytes.len() * 2,
        });
        request_overflow.authorize("cmap", "UniKS-UCS2-H").unwrap();
        assert_eq!(
            request_overflow
                .authorize("cmap", "UniKS-UCS2-H")
                .unwrap_err(),
            ResourceFailure::LimitExceeded
        );
    }
}
