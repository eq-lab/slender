use crate::{tests::sut::init_pool, *};

#[test]
fn should_return_paused_flag() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    sut.pool.set_pause(&false);
    assert!(!sut.pool.paused());

    sut.pool.set_pause(&true);
    assert!(sut.pool.paused());
}
