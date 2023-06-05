use crate::mock::*;
use frame_support::{assert_noop, assert_ok};

#[test]
fn set_reference() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_ok!(TemplateModule::set_reference(
            RuntimeOrigin::signed(1),
            vec![b'h', b'e', b'l', b'l', b'o'],
            vec![b'w', b'o', b'r', b'l', b'd']
        ));
        // Read pallet storage and assert an expected result.
        assert_eq!(
            TemplateModule::references(vec![b'h', b'e', b'l', b'l', b'o']),
            vec![b'w', b'o', b'r', b'l', b'd']
        );
    });
}

#[test]
fn get_reference() {
    new_test_ext().execute_with(|| {
        assert_ok!(TemplateModule::set_reference(
            RuntimeOrigin::signed(1),
            vec![b'h', b'e', b'l', b'l', b'o'],
            vec![b'w', b'o', b'r', b'l', b'd']
        ));
        assert_ok!(TemplateModule::get_reference(
            RuntimeOrigin::signed(2),
            vec![b'h', b'e', b'l', b'l', b'o']
        ));
    });
}

#[test]
fn remove_key() {
    new_test_ext().execute_with(|| {
        assert_ok!(TemplateModule::set_reference(
            RuntimeOrigin::signed(1),
            vec![b'h', b'e', b'l', b'l', b'o'],
            vec![b'w', b'o', b'r', b'l', b'd']
        ));
        assert_ok!(TemplateModule::remove_reference(
            RuntimeOrigin::signed(2),
            vec![b'h', b'e', b'l', b'l', b'o']
        ));
    });
}
