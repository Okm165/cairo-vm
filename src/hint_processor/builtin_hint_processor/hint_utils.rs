use crate::stdlib::{borrow::Cow, collections::HashMap, prelude::*};

use felt::Felt252;

use crate::hint_processor::hint_processor_definition::HintReference;
use crate::hint_processor::hint_processor_utils::{
    compute_addr_from_reference, get_ptr_from_reference,
};
use crate::hint_processor::hint_processor_utils::{
    get_integer_from_reference, get_maybe_relocatable_from_reference,
};
use crate::serde::deserialize_program::ApTracking;
use crate::types::relocatable::MaybeRelocatable;
use crate::types::relocatable::Relocatable;
use crate::vm::errors::hint_errors::HintError;
use crate::vm::vm_core::VirtualMachine;

//Inserts value into the address of the given ids variable
pub fn insert_value_from_var_name(
    var_name: &str,
    value: impl Into<MaybeRelocatable>,
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let var_address = get_relocatable_from_var_name(var_name, vm, ids_data, ap_tracking)?;
    vm.insert_value(var_address, value)
        .map_err(HintError::Memory)
}

//Inserts value into ap
pub fn insert_value_into_ap(
    vm: &mut VirtualMachine,
    value: impl Into<MaybeRelocatable>,
) -> Result<(), HintError> {
    vm.insert_value(vm.get_ap(), value)
        .map_err(HintError::Memory)
}

//Returns the Relocatable value stored in the given ids variable
pub fn get_ptr_from_var_name(
    var_name: &str,
    vm: &VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<Relocatable, HintError> {
    let reference = get_reference_from_var_name(var_name, ids_data)?;
    match get_ptr_from_reference(vm, reference, ap_tracking) {
        // Map internal errors into more descriptive variants
        Ok(val) => Ok(val),
        Err(HintError::WrongIdentifierTypeInternal(var_addr)) => Err(
            HintError::IdentifierNotRelocatable(var_name.to_string(), var_addr),
        ),
        _ => Err(HintError::UnknownIdentifier(var_name.to_string())),
    }
}

//Gets the address, as a Relocatable of the variable given by the ids name
pub fn get_relocatable_from_var_name(
    var_name: &str,
    vm: &VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<Relocatable, HintError> {
    ids_data
        .get(var_name)
        .and_then(|x| compute_addr_from_reference(x, vm, ap_tracking))
        .ok_or_else(|| HintError::UnknownIdentifier(var_name.to_string()))
}

//Gets the value of a variable name.
//If the value is an MaybeRelocatable::Int(Bigint) return &Bigint
//else raises Err
pub fn get_integer_from_var_name<'a>(
    var_name: &'a str,
    vm: &'a VirtualMachine,
    ids_data: &'a HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<Cow<'a, Felt252>, HintError> {
    let reference = get_reference_from_var_name(var_name, ids_data)?;
    match get_integer_from_reference(vm, reference, ap_tracking) {
        // Map internal errors into more descriptive variants
        Ok(val) => Ok(val),
        Err(HintError::WrongIdentifierTypeInternal(var_addr)) => Err(
            HintError::IdentifierNotInteger(var_name.to_string(), var_addr),
        ),
        _ => Err(HintError::UnknownIdentifier(var_name.to_string())),
    }
}

//Gets the value of a variable name as a MaybeRelocatable
pub fn get_maybe_relocatable_from_var_name<'a>(
    var_name: &str,
    vm: &'a VirtualMachine,
    ids_data: &'a HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<MaybeRelocatable, HintError> {
    let reference = get_reference_from_var_name(var_name, ids_data)?;
    get_maybe_relocatable_from_reference(vm, reference, ap_tracking)
        .ok_or_else(|| HintError::UnknownIdentifier(var_name.to_string()))
}

pub fn get_reference_from_var_name<'a>(
    var_name: &'a str,
    ids_data: &'a HashMap<String, HintReference>,
) -> Result<&'a HintReference, HintError> {
    ids_data
        .get(var_name)
        .ok_or(HintError::UnknownIdentifier(var_name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::string::ToString;
    use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
    use crate::{
        hint_processor::hint_processor_definition::HintReference,
        relocatable,
        serde::deserialize_program::OffsetValue,
        utils::test_utils::*,
        vm::{
            errors::memory_errors::MemoryError, vm_core::VirtualMachine, vm_memory::memory::Memory,
        },
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_ptr_from_var_name_immediate_value() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (0, 0))];
        let mut hint_ref = HintReference::new(0, 0, true, false);
        hint_ref.offset2 = OffsetValue::Value(2);
        let ids_data = HashMap::from([("imm".to_string(), hint_ref)]);

        assert_matches!(
            get_ptr_from_var_name("imm", &vm, &ids_data, &ApTracking::new()),
            Ok(x) if x == relocatable!(0, 2)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_maybe_relocatable_from_var_name_valid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_maybe_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(x) if x == mayberelocatable!(0, 0)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_maybe_relocatable_from_var_name_invalid() {
        let mut vm = vm!();
        vm.segments.memory = Memory::new();
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_maybe_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::UnknownIdentifier(x)) if x == *"value"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_ptr_from_var_name_valid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_ptr_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(x) if x == relocatable!(0, 0)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_ptr_from_var_name_invalid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 0)];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_ptr_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::IdentifierNotRelocatable(x,y
            )) if x == "value" && y == (1,0).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_relocatable_from_var_name_valid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(x) if x == relocatable!(1, 0)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_relocatable_from_var_name_invalid() {
        let mut vm = vm!();
        vm.segments.memory = Memory::new();
        let hint_ref = HintReference::new_simple(-8);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::UnknownIdentifier(x)) if x == "value"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_integer_from_var_name_valid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 1)];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_integer_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(Cow::Borrowed(x)) if x == &Felt252::new(1)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_integer_from_var_name_invalid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_matches!(
            get_integer_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "value" && y == (1,0).into()
        );
    }
}
