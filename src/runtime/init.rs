use crate::builtin::console::log;
use crate::builtin::fetch::fetch;

use super::Runtime;

impl Runtime {
    pub fn init_global<'s>(scope: &mut v8::HandleScope<'s, ()>) -> v8::Local<'s, v8::Context> {
        let scope = &mut v8::EscapableHandleScope::new(scope);

        let context = v8::Context::new(scope);
        let global = context.global(scope);
        let scope = &mut v8::ContextScope::new(scope, context);

        Self::set_func(scope, global, "fetch", fetch);

        let console_key = v8::String::new(scope, "console").unwrap();
        let console_object = v8::Object::new(scope);
        global.set(scope, console_key.into(), console_object.into());

        Self::set_func(scope, console_object, "log", log);
        Self::set_func(scope, console_object, "info", log);
        Self::set_func(scope, console_object, "error", log);
        Self::set_func(scope, console_object, "warn", log);

        scope.escape(context)
    }

    pub fn set_func(
        scope: &mut v8::HandleScope,
        obj: v8::Local<v8::Object>,
        name: &str,
        func: impl v8::MapFnTo<v8::FunctionCallback>,
    ) {
        let tml = v8::FunctionTemplate::new(scope, func);

        let func = tml.get_function(scope).unwrap();
        let func_name = v8::String::new(scope, name).unwrap();
        func.set_name(func_name);
        obj.set(scope, func_name.into(), func.into());
    }

    pub fn set_obj(
        scope: &mut v8::HandleScope,
        obj: v8::Local<v8::Object>,
        name: &str,
        val: v8::Local<v8::Object>,
    ) {
        let key = v8::String::new(scope, name).unwrap();
        obj.set(scope, key.into(), val.into());
    }
}
