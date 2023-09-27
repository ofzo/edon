use chrono::{TimeZone, Utc};
use colored::Colorize;

pub fn console_format(
    scope: &mut v8::HandleScope,
    v: &v8::Local<v8::Value>,
    level: usize,
) -> String {
    if v.is_boolean() {
        return format!("{}", v.boolean_value(scope).to_string().magenta());
    }
    if v.is_string() {
        if level == 0 {
            return format!(r#"{}"#, v.to_rust_string_lossy(scope));
        }
        return format!(
            "{}",
            format!(r#""{}""#, v.to_rust_string_lossy(scope)).green()
        );
    }
    if v.is_null() {
        return format!("{}", "NULL".color("gray"));
    }
    if v.is_undefined() {
        return format!("{}", "undefined".color("gray"));
    }
    if v.is_number() {
        return format!("{}", v.number_value(scope).unwrap().to_string().red());
    }
    if v.is_big_int() {
        let bigint = v8::Local::<v8::BigInt>::try_from(*v).unwrap();
        return format!("{}", format!("{}n", bigint.u64_value().0.to_string()).red());
    }
    if v.is_array() {
        let array = v8::Local::<v8::Array>::try_from(*v).unwrap();
        let fmt = (0..array.length())
            .map(|index| {
                let val = array.get_index(scope, index).unwrap();
                format!("{}", console_format(scope, &val, level + 1))
            })
            .collect::<Vec<_>>()
            .join(",");
        return format!("[ {fmt} ]");
    }
    if v.is_function() {
        let func = v8::Local::<v8::Function>::try_from(*v).unwrap();
        let name = func
            .get_name(scope)
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope);

        let name = if name.is_empty() {
            format!("Anonymous")
        } else {
            name
        };
        let tag = if v.is_async_function() {
            "AsyncFunction"
        } else if v.is_generator_function() {
            "GeneratorFunction"
        } else {
            "Function"
        };
        return format!("{}", format!("[ {tag} <{name}> ]").yellow());
    }
    if v.is_promise() {
        let promise = v8::Local::<v8::Promise>::try_from(*v).unwrap();
        return match promise.state() {
            v8::PromiseState::Pending => format!("{}", "[[ Promise <Pending> ]]".magenta()),
            v8::PromiseState::Fulfilled => format!("{}", "[[ Promise <Resolved> ]]".magenta()),
            v8::PromiseState::Rejected => format!("{}", "[[ Promise <Rejected> ]]".red()),
        };
    }
    if v.is_date() {
        let date = v8::Local::<v8::Date>::try_from(*v).unwrap();
        let val = date.number_value(scope).unwrap();
        let date = Utc.timestamp_nanos((val * 1000_000.0).floor() as i64);
        return format!("{}", date.to_rfc3339().magenta());
    }
    if v.is_object() {
        let obj = v8::Local::<v8::Object>::try_from(*v).unwrap();
        let names = obj
            .get_own_property_names(scope, Default::default())
            .unwrap();
        // let prototype = obj.get_prototype(scope).unwrap();
        let mut fmt = (0..names.length())
            .map(|index| {
                let name = names.get_index(scope, index).unwrap();
                let val = obj.get(scope, name).unwrap();
                format!(
                    "{}{}: {}",
                    "  ".repeat(level + 1).to_string(),
                    console_format(scope, &name, level + 1),
                    console_format(scope, &val, level + 1)
                )
            })
            .collect::<Vec<_>>();
        fmt.push(format!(
            "{}{}: {{}}",
            "  ".repeat(level + 1).to_string(),
            "[[prototype]]".color("gray"),
            // console_format(scope, &prototype, level + 1)
        ));
        return format!(
            "{{\n{}\n{}}}",
            fmt.join(",\n"),
            "  ".repeat(level).to_string()
        );
    }
    format!("")
}

pub fn console_log(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _rv: v8::ReturnValue,
) {
    let result = (0..args.length())
        .into_iter()
        .map(|index| {
            let val = args.get(index);
            console_format(scope, &val, 0)
        })
        .collect::<Vec<_>>()
        .join(" ");

    println!("{result}");
}
