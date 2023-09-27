use std::path::Path;

use std::fs;

use crate::runtime::EdonRuntime;

fn read(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _rv: v8::ReturnValue,
) {
    if let Ok(v) = v8::Local::<v8::String>::try_from(args.get(0)) {
        let p = v.to_rust_string_lossy(scope);
        let path = Path::new(&p);
        let content = fs::read(path).unwrap();
        let content = String::from_utf8(content).unwrap();
        if let Ok(f) = v8::Local::<v8::Function>::try_from(args.get(1)) {
            let undefined = v8::undefined(scope).into();
            let content = v8::String::new(scope, &content).unwrap();
            f.call(scope, undefined, &[content.into()]).unwrap();
        }
        // _rv.set(v8::String::new(scope, &content).unwrap().into());
    }
}
fn read_sync(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _rv: v8::ReturnValue,
) {
    if let Ok(v) = v8::Local::<v8::String>::try_from(args.get(0)) {
        let p = v.to_rust_string_lossy(scope);
        let path = Path::new(&p);
        let content = fs::read(path).unwrap();
        let content = String::from_utf8(content).unwrap();
        _rv.set(v8::String::new(scope, &content).unwrap().into());
    }
}

fn read_promise(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _rv: v8::ReturnValue,
) {
    if let Ok(v) = v8::Local::<v8::String>::try_from(args.get(0)) {
        let p = v.to_rust_string_lossy(scope);
        let path = Path::new(&p);

        let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
        let promise = promise_resolver.get_promise(scope);
        _rv.set(promise.into());
        // tokio::spawn(async move {
        let content = fs::read(path);
        if let Ok(content) = content {
            let content = String::from_utf8(content).unwrap();
            let content = v8::String::new(scope, &content).unwrap();
            promise_resolver.resolve(scope, content.into());
        } else {
            let error = v8::String::new(scope, "").unwrap();
            promise_resolver.reject(scope, error.into());
        }
        // });
        // let handler = tokio::spawn(tokio::fs::read(path));
        // tokio::join!(handler);

        // let content = fs::read(path).unwrap();
        // let content = String::from_utf8(content).unwrap();

        // promise_resolver.resolve(scope, value);
    }
}

pub fn init<'s>(scope: &mut v8::HandleScope<'s>) -> v8::Local<'s, v8::Object> {
    let obj = v8::Object::new(scope);
    EdonRuntime::set_func(scope, obj, "read", read);
    EdonRuntime::set_func(scope, obj, "readSync", read_sync);
    EdonRuntime::set_func(scope, obj, "readPromise", read_promise);
    return obj;
}
