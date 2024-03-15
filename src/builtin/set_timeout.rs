use std::{thread, time};

pub fn set_timeout(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let callback = args.get(0);
    let timeout = args
        .get(1)
        .to_number(scope)
        .expect("timeout must be an number")
        .int32_value(scope)
        .expect("timeout must be an number");
    let arg = args.get(2);

    if callback.is_function() {
        let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
        let timeout = timeout.max(0) as u64;
        // let (sender, receiver) = oneshot::channel::<bool>();
        thread::sleep(time::Duration::from_millis(timeout));
        // thread::spawn(move || {
        //     let _ = sender.send(true);
        // });
        callback.call(scope, args.this().into(), &[arg]);
    }
    rv.set_int32(123);
}

#[inline]
pub fn set_interval<'scope>(
    scope: &mut v8::HandleScope<'scope>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let callback = args.get(0);
    let timeout = args
        .get(1)
        .to_number(scope)
        .expect("timeout must be an number")
        .int32_value(scope)
        .expect("timeout must be an number");
    let arg = args.get(2);
    if callback.is_function() {
        let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
        let timeout = timeout.max(0) as u64;
        loop {
            let ts_scope = &mut v8::TryCatch::new(scope);
            thread::sleep(time::Duration::from_millis(timeout));
            callback.call(ts_scope, args.this().into(), &[arg]);
            if ts_scope.has_caught() {
                println!("catch err, {:?}", ts_scope.exception().unwrap());
                break;
            }
        }
    }
    rv.set_int32(123);
}
