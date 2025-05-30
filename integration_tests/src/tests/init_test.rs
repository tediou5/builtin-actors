use export_macro::vm_test;
use fil_actor_init::Exec4Return;
use fil_actors_runtime::{
    EAM_ACTOR_ADDR, EAM_ACTOR_ID, INIT_ACTOR_ADDR, cbor::serialize, runtime::EMPTY_ARR_CID,
    test_utils::MULTISIG_ACTOR_CODE_ID,
};
use fvm_shared::{METHOD_SEND, address::Address, econ::TokenAmount, error::ExitCode};
use num_traits::Zero;
use vm_api::{VM, builtin::Type, util::serialize_ok};

use crate::{FIRST_TEST_USER_ADDR, TEST_FAUCET_ADDR};

fn assert_placeholder_actor(exp_bal: TokenAmount, v: &dyn VM, addr: Address) {
    let act = v.actor(&addr).unwrap();
    assert_eq!(EMPTY_ARR_CID, act.state);
    assert_eq!(&Type::Placeholder, v.actor_manifest().get(&act.code).unwrap());
    assert_eq!(exp_bal, act.balance);
}

#[vm_test]
pub fn placeholder_deploy_test(v: &dyn VM) {
    // Create a placeholder.
    let subaddr = b"foobar";
    let addr = Address::new_delegated(EAM_ACTOR_ID, subaddr).unwrap();
    assert!(
        v.execute_message(
            &TEST_FAUCET_ADDR,
            &addr,
            &TokenAmount::from_atto(42u8),
            METHOD_SEND,
            None,
        )
        .unwrap()
        .code
        .is_success()
    );
    let expect_id_addr = Address::new_id(FIRST_TEST_USER_ADDR);
    assert_placeholder_actor(TokenAmount::from_atto(42u8), v, expect_id_addr);

    // Make sure we assigned the right f4 address.
    assert_eq!(v.resolve_id_address(&addr).unwrap(), expect_id_addr);

    // Deploy a multisig to the placeholder.
    let msig_ctor_params = serialize(
        &fil_actor_multisig::ConstructorParams {
            signers: vec![EAM_ACTOR_ADDR],
            num_approvals_threshold: 1,
            unlock_duration: 0,
            start_epoch: 0,
        },
        "multisig ctor params",
    )
    .unwrap();

    let deploy = || {
        v.execute_message(
            &EAM_ACTOR_ADDR, // so this works even if "m2-native" is disabled.
            &INIT_ACTOR_ADDR,
            &TokenAmount::zero(),
            fil_actor_init::Method::Exec4 as u64,
            Some(serialize_ok(&fil_actor_init::Exec4Params {
                code_cid: *MULTISIG_ACTOR_CODE_ID,
                constructor_params: msig_ctor_params.clone(),
                subaddress: subaddr[..].to_owned().into(),
            })),
        )
        .unwrap()
    };

    let msig_ctor_res = deploy();
    assert_eq!(msig_ctor_res.code, ExitCode::OK);
    let msig_ctor_ret: Exec4Return = msig_ctor_res.ret.unwrap().deserialize().unwrap();

    assert_eq!(
        expect_id_addr, msig_ctor_ret.id_address,
        "expected actor to be deployed over placeholder"
    );

    // Make sure we kept the balance.
    assert_eq!(v.actor(&expect_id_addr).unwrap().balance, TokenAmount::from_atto(42u8));

    // Try to overwrite it.
    let msig_ctor_res = deploy();
    assert_eq!(ExitCode::USR_FORBIDDEN, msig_ctor_res.code);
}
