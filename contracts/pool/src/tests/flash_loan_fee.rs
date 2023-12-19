use crate::{tests::sut::init_pool, *};

#[test]
fn should_return_flash_loan_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    assert_eq!(sut.pool.flash_loan_fee(), 5);
}
