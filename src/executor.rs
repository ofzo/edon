use v8;

fn require(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _rv: v8::ReturnValue,
) {
    if let Ok(v) = v8::Local::<v8::String>::try_from(args.get(0)) {
        println!("require!!:: {}", v.to_rust_string_lossy(scope));
        _rv.set(v8::String::new(scope, "ok").unwrap().into());
    }
}

fn log(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut _rv: v8::ReturnValue) {
    for i in 0..args.length() {
        if let Ok(v) = v8::Local::<v8::String>::try_from(args.get(i)) {
            print!("{}", v.to_rust_string_lossy(scope));
            // _rv.set(v8::String::new(scope, "ok").unwrap().into());
        }
        if let Ok(v) = v8::Local::<v8::Boolean>::try_from(args.get(i)) {
            print!("{}", v.to_rust_string_lossy(scope));
            // _rv.set(v8::String::new(scope, "ok").unwrap().into());
        }
        if let Ok(v) = v8::Local::<v8::Number>::try_from(args.get(i)) {
            print!("{}", v.to_rust_string_lossy(scope));
            // _rv.set(v8::String::new(scope, "ok").unwrap().into());
        }
        print!(" ");
    }
    print!("\n");
}

fn set_func(
    scope: &mut v8::HandleScope,
    obj: v8::Local<v8::Object>,
    name: &str,
    func: impl v8::MapFnTo<v8::FunctionCallback>,
) {
    let tml = v8::FunctionTemplate::new(scope, func);

    let val = tml.get_function(scope).unwrap();
    let print_name = v8::String::new(scope, name).unwrap();
    val.set_name(print_name);
    obj.set(scope, print_name.into(), val.into());
}

pub fn run<'a>(code: String) {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);

    let scope = &mut v8::EscapableHandleScope::new(scope);

    let context = v8::Context::new(scope);
    let global = context.global(scope);

    let scope = &mut v8::ContextScope::new(scope, context);
    let object = v8::Object::new(scope);

    let console_key = v8::String::new(scope, "console").unwrap().into();
    global.set(scope, console_key, object.into());

    set_func(scope, global, "require", require);

    let console_val = v8::Object::new(scope);
    let console_key = v8::String::new(scope, "console").unwrap().into();
    global.set(scope, console_key, console_val.into());
    set_func(scope, console_val, "log", log);

    let exports_val = v8::Object::new(scope);
    let exports_key = v8::String::new(scope, "exports").unwrap().into();
    global.set(scope, exports_key, exports_val.into());

    let code = v8::String::new(scope, &code).unwrap();
    // println!("javascript code: {}", code.to_rust_string_lossy(scope));

    let script = v8::Script::compile(scope, code, None).unwrap();
    script.run(scope).unwrap();
    let names = exports_val.get_property_names(scope).unwrap();
    // let exports = exports_val.to_string(scope).unwrap();
    println!("javascript result: {}", names.to_rust_string_lossy(scope));

    // exports_val
}
