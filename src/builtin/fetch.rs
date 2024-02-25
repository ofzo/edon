use crate::runtime::AsynchronousKind;
use std::{str::FromStr, sync::Arc, task::Poll};

use core::pin::Pin;
use reqwest::{self, Client, Method};

use crate::runtime::Runtime;

#[derive(Debug, Default)]
struct FetchOption {
    // A BodyInit object or null to set request's body.
    // body: Option<BodyInit>,
    // A string indicating how the request will interact with the browser's cache to set request's cache.
    // cache: Option<RequestCache>,
    // A string indicating whether credentials will be sent with the request always, never, or only when sent to a same-origin URL. Sets request's credentials.
    // credentials: Option<RequestCredentials>,
    // A Headers object, an object literal, or an array of two-item arrays to set request's headers.
    // headers: Option<HeadersInit>,
    // A cryptographic hash of the resource to be fetched by request. Sets request's integrity.
    // integrity: Option<String>,
    // A boolean to set request's keepalive.
    // keepalive: Option<bool>,
    // A string to set request's method.
    method: Method,
    // A string to indicate whether the request will use CORS, or will be restricted to same-origin URLs. Sets request's mode.
    // mode: Option<RequestMode>,
    // A string indicating whether request follows redirects, results in an error upon encountering a redirect, or returns the redirect (in an opaque fashion). Sets request's redirect.
    // redirect: Option<RequestRedirect>,
    // A string whose value is a same-origin URL, "about:client", or the empty string, to set request's referrer.
    // referrer: Option<String>,
    // A referrer policy to set request's referrerPolicy.
    // referrer_policy: Option<ReferrerPolicy>,
    // An AbortSignal to set request's signal.
    // signal: Option<AbortSignal>,
}

pub fn fetch(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let state_rc = Runtime::state(scope);

    let url = args.get(0);
    let options = args.get(1);

    let init = if options.is_object() {
        let obj = v8::Local::<v8::Object>::try_from(options).unwrap();

        let mut init = FetchOption::default();

        let method_key = v8::String::new(scope, "method").unwrap();
        init.method = if let Some(method) = obj.get(scope, method_key.into()) {
            Method::from_str(method.to_rust_string_lossy(scope).as_str()).expect("invalida method")
        } else {
            Method::GET
        };
        init
    } else {
        FetchOption::default()
    };
    let client = reqwest::Client::new();
    let builder = client.request(init.method, url.to_rust_string_lossy(scope));

    let resolver = v8::PromiseResolver::new(scope).unwrap();

    // let resolver = Arc::new(resolver);

    let state = state_rc.borrow_mut();
    // let move_resolver = resolver.clone();
    // state.pending_ops.push(Pin::new(Poll::Ready(AsynchronousKind::Callback(async move {
    //     let builder = builder.build().unwrap();

    //     let response = Client::execute(&client, builder);

    //     let status = response.status();
    //     let value = v8::ObjectTemplate::new(scope);
    //     let status_key = v8::String::new(scope, "status").unwrap();
    //     let status_value = v8::String::new(scope, status.as_str()).unwrap();
    //     value.set(status_key.into(), status_value.into());

    //     let value = value.new_instance(scope).unwrap();
    //     move_resolver.resolve(scope, value.into());

    // }))));

    rv.set(resolver.into());
}
