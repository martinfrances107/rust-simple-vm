
use simplevm::Register::*;

mod common;
use common::*;

#[test]
fn addr_of_global() {
   let test = "
global int test;
global int foo;

void main() {
    let *int b := &foo;
    return b;
}
   ";
    let vm = run_program(test).unwrap();
    assert_eq!(vm.get_register(A), 2);
}

#[test]
fn assign_deref() {
   let test = "
global int foo;

void main() {
    let *int b := &foo;
    *(b) := 77;
    return foo;
}
   ";
    let vm = run_program(test).unwrap();
    assert_eq!(vm.get_register(A), 77);
}

