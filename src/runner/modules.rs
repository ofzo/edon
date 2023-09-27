pub fn native_module_inject<'a>(
    context: v8::Local<'a, v8::Context>,
    module: v8::Local<v8::Module>,
) -> Option<v8::Local<'a, v8::Value>> {
    let scope = unsafe { &mut v8::CallbackScope::new(context) };
    let tc_scope = &mut v8::TryCatch::new(scope);

    // let resolver = v8::PromiseResolver::new(tc_scope).unwrap();
    // define default to function
    let default_name = v8::String::new(tc_scope, "default").unwrap();
    let func = v8::FunctionTemplate::new(tc_scope, empty_function);
    let func = func.get_function(tc_scope).unwrap();
    func.set_name(v8::String::new(tc_scope, "default").unwrap());

    // define hello to object
    let hello_name = v8::String::new(tc_scope, "hello").unwrap();
    let hello = v8::ObjectTemplate::new(tc_scope);
    // hello = { hello :"world" }
    let key = v8::String::new(tc_scope, "hello").unwrap();
    let value = v8::String::new(tc_scope, "world").unwrap();
    hello.set(key.into(), value.into());
    let hello = hello.new_instance(tc_scope).unwrap().into();

    let _result = module.set_synthetic_module_export(tc_scope, default_name, func.into());
    let _result = module.set_synthetic_module_export(tc_scope, hello_name, hello);

    let resolver = v8::PromiseResolver::new(tc_scope).unwrap();
    let undefined = v8::undefined(tc_scope);
    resolver.resolve(tc_scope, undefined.into());
    let promise = resolver.get_promise(tc_scope);

    Some(promise.into())
}

pub fn empty_function(
    scope: &mut v8::HandleScope,
    _info: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    rv.set(v8::String::new(scope, "hello").unwrap().into());
}
