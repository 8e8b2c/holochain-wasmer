pub mod import;
pub mod load_wasm;

extern crate holochain_json_api;

use holochain_json_api::json::RawString;
use holochain_wasmer_host::guest;
use holochain_wasmer_host::*;
use test_common::SomeStruct;
use wasmer_runtime::Ctx;

fn test_process_string(
    ctx: &mut Ctx,
    allocation_ptr: AllocationPtr,
) -> Result<AllocationPtr, WasmError> {
    let processed_string: RawString = guest::from_allocation_ptr(ctx, allocation_ptr)?;
    let processed_string = format!("host: {}", String::from(processed_string));
    Ok(holochain_wasmer_host::json::to_allocation_ptr(
        RawString::from(processed_string).into(),
    ))
}

fn test_process_struct(
    ctx: &mut Ctx,
    allocation_ptr: AllocationPtr,
) -> Result<AllocationPtr, WasmError> {
    let mut some_struct: SomeStruct = guest::from_allocation_ptr(ctx, allocation_ptr)?;
    some_struct.process();
    Ok(holochain_wasmer_host::json::to_allocation_ptr(
        some_struct.into(),
    ))
}

#[cfg(test)]
pub mod tests {

    use crate::import::import_object;
    use crate::load_wasm::load_wasm;
    use holochain_json_api::json::RawString;
    use holochain_wasmer_host::guest;
    use holochain_wasmer_host::*;
    use test_common::SomeStruct;
    use wasmer_runtime::instantiate;
    use wasmer_runtime::Instance;

    fn test_instance() -> Instance {
        instantiate(&load_wasm(), &import_object()).expect("build test instance")
    }

    #[test]
    fn bytes_test() {
        let result: Vec<u8> = guest::call_bytes(
            &mut test_instance(),
            "process_bytes",
            vec![0_u8, 1_u8, 2_u8, 3_u8],
        )
        .expect("bytes call");

        assert_eq!(
            vec![0_u8, 1_u8, 2_u8, 3_u8, 50_u8, 60_u8, 70_u8, 80_u8],
            result,
        );
    }

    #[test]
    fn stacked_test() {
        let result: RawString =
            guest::call(&mut test_instance(), "stacked_strings", ()).expect("stacked strings call");

        assert_eq!("first", &String::from(result));
    }

    #[test]
    fn process_string_test() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((10_u32 * std::u16::MAX as u32) as _);

        let result: RawString = guest::call(
            &mut test_instance(),
            "process_string",
            RawString::from(starter_string.clone()),
        )
        .expect("process string call");

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(&String::from(result), &expected_string,);
    }

    #[test]
    fn native_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct =
            guest::call(&mut test_instance(), "native_type", some_struct.clone())
                .expect("native type handling");

        assert_eq!(some_struct, result,);
    }

    #[test]
    fn native_struct_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct =
            guest::call(&mut test_instance(), "process_native", some_struct.clone()).unwrap();

        let expected = SomeStruct::new(format!("processed: {}", some_inner));
        assert_eq!(result, expected,);
    }

    #[test]
    fn ret_test() {
        let result: Result<SomeStruct, WasmError> =
            guest::call(&mut test_instance(), "some_ret", ());
        match result {
            Ok(some_struct) => {
                assert_eq!(SomeStruct::new("foo".into()), some_struct,);
            }
            Err(_) => unreachable!(),
        };

        let err: Result<SomeStruct, WasmError> =
            guest::call(&mut test_instance(), "some_ret_err", ());
        match err {
            Err(wasm_error) => assert_eq!(WasmError::Zome("oh no!".into()), wasm_error,),
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn try_result_test() {
        let success_result: Result<SomeStruct, WasmError> =
            guest::call(&mut test_instance(), "try_result_succeeds", ());
        match success_result {
            Ok(some_struct) => {
                assert_eq!(SomeStruct::new("foo".into()), some_struct,);
            }
            Err(_) => unreachable!(),
        };

        let fail_result: Result<(), WasmError> =
            guest::call(&mut test_instance(), "try_result_fails_fast", ());
        match fail_result {
            Err(wasm_error) => {
                assert_eq!(WasmError::Zome("it fails!".into()), wasm_error,);
            }
            Ok(_) => unreachable!(),
        };
    }
}
