//! Compatibility globals required by the pinned PDF.js modules.

pub(super) const SHIMS: &str = r#"
globalThis.DOMException ??= class DOMException extends Error { constructor(message, name='Error') { super(message); this.name=name; } };
globalThis.AbortSignal ??= class AbortSignal {
  constructor(){this.aborted=false;this.reason=undefined;this.listeners=[];}
  addEventListener(type,fn){if(type==='abort')this.listeners.push(fn);}
  removeEventListener(type,fn){if(type==='abort')this.listeners=this.listeners.filter(x=>x!==fn);}
  throwIfAborted(){if(this.aborted)throw this.reason||new DOMException('Aborted','AbortError');}
};
globalThis.AbortController ??= class AbortController {
  constructor(){this.signal=new AbortSignal();}
  abort(reason){if(this.signal.aborted)return;this.signal.aborted=true;this.signal.reason=reason||new DOMException('Aborted','AbortError');for(const fn of this.signal.listeners)fn.call(this.signal,{type:'abort',target:this.signal});}
};
globalThis.ReadableStream ??= class ReadableStream {
  constructor(source, options={}) {
    this.source=source||{}; this.locked=false; this.queue=[]; this.waiters=[]; this.closed=false; this.failure=null; this.pulling=false;
    this.controller={
      enqueue:chunk=>{if(this.closed)throw new TypeError('stream is closed');const waiter=this.waiters.shift();if(waiter)waiter({value:chunk,done:false});else this.queue.push(chunk);},
      close:()=>{this.closed=true;for(const waiter of this.waiters.splice(0))waiter({value:undefined,done:true});},
      error:error=>{this.failure=error;this.closed=true;for(const waiter of this.waiters.splice(0))waiter(Promise.reject(error));},
      get desiredSize(){return options.highWaterMark??1;}
    };
    if(typeof this.source.start==='function') { const result=this.source.start(this.controller); if(result&&typeof result.catch==='function')result.catch(e=>this.controller.error(e)); }
  }
  getReader(){
    if(this.locked)throw new TypeError('stream already has a reader'); this.locked=true; const stream=this;
    return {read(){
      if(stream.failure)return Promise.reject(stream.failure);
      if(stream.queue.length)return Promise.resolve({value:stream.queue.shift(),done:false});
      if(stream.closed)return Promise.resolve({value:undefined,done:true});
      const result=new Promise((resolve,reject)=>stream.waiters.push(value=>value instanceof Promise?value.then(resolve,reject):resolve(value)));
      if(!stream.pulling&&typeof stream.source.pull==='function'){
        stream.pulling=true; let pulled;
        try{pulled=stream.source.pull(stream.controller);}catch(e){stream.controller.error(e);}
        Promise.resolve(pulled).finally(()=>{stream.pulling=false;});
      }
      return result;
    },cancel(reason){stream.controller.close();return stream.source.cancel?Promise.resolve(stream.source.cancel(reason)):Promise.resolve();},releaseLock(){stream.locked=false;}};
  }
  cancel(reason){this.controller.close();return this.source.cancel?Promise.resolve(this.source.cancel(reason)):Promise.resolve();}
  tee(){throw new TypeError('stream tee is unsupported');}
};
globalThis.URLSearchParams ??= class URLSearchParams {
  constructor(value='') { this.map=new Map(); for(const part of String(value).replace(/^\?/,'').split('&')) { if(!part) continue; const [k,v='']=part.split('='); this.append(decodeURIComponent(k.replace(/\+/g,' ')),decodeURIComponent(v.replace(/\+/g,' '))); } }
  append(k,v){const a=this.map.get(String(k))||[];a.push(String(v));this.map.set(String(k),a);}
  set(k,v){this.map.set(String(k),[String(v)]);}
  get(k){return this.map.get(String(k))?.[0]??null;}
  getAll(k){return [...(this.map.get(String(k))||[])];}
  has(k){return this.map.has(String(k));}
  delete(k){this.map.delete(String(k));}
  forEach(fn,thisArg){for(const [k,v] of this) fn.call(thisArg,v,k,this);}
  sort(){const all=[...this].sort((a,b)=>a[0]<b[0]?-1:a[0]>b[0]?1:0);this.map=new Map();for(const [k,v] of all)this.append(k,v);}
  get size(){let n=0;for(const [,vs] of this.map)n+=vs.length;return n;}
  toString(){return [...this.map].flatMap(([k,vs])=>vs.map(v=>encodeURIComponent(k)+'='+encodeURIComponent(v))).join('&');}
  *entries(){for(const [k,vs] of this.map) for(const v of vs) yield [k,v];}
  [Symbol.iterator](){return this.entries();}
};
globalThis.URL ??= class URL {
  constructor(value, base) {
    const raw=String(value); const b=base ? String(base) : '';
    const full=/^[a-z][a-z0-9+.-]*:/i.test(raw) ? raw : b+raw;
    const m=/^([a-z][a-z0-9+.-]*:)?(?:\/\/([^\/?#]*))?([^?#]*)(\?[^#]*)?(#.*)?$/i.exec(full);
    if(!m) throw new TypeError('Invalid URL');
    this.href=full; this.protocol=m[1]||''; this.host=m[2]||''; this.hostname=this.host.split(':')[0];
    this.pathname=m[3]||''; this.search=m[4]||''; this.hash=m[5]||'';
    this.origin=this.protocol&&this.host ? this.protocol+'//'+this.host : 'null';
    this.searchParams=new URLSearchParams(this.search);
  }
  toString(){return this.href;}
};
globalThis.structuredClone ??= function(value) {
  const seen = new Map();
  function copy(v) {
    if (v === null || typeof v !== 'object') return v;
    if (seen.has(v)) return seen.get(v);
    if (v instanceof ArrayBuffer) return v.slice(0);
    if (ArrayBuffer.isView(v)) return new v.constructor(v);
    if (Array.isArray(v)) { const a=[]; seen.set(v,a); for (const x of v) a.push(copy(x)); return a; }
    const o={}; seen.set(v,o); for (const k of Object.keys(v)) o[k]=copy(v[k]); return o;
  }
  return copy(value);
};
globalThis.__pdfjsCMapReaderFactory = class {
  constructor({isCompressed=true}={}) { this.isCompressed=isCompressed; }
  async fetch({name}) {
    if (!this.isCompressed) throw new TypeError('only packed PDF.js CMaps are embedded');
    return {cMapData:globalThis.__pdfjsReadEmbeddedResource('cmap',name),isCompressed:true};
  }
};
globalThis.__pdfjsStandardFontDataFactory = class {
  async fetch({filename}) {
    return globalThis.__pdfjsReadEmbeddedResource('standard_font',filename);
  }
};
"#;
