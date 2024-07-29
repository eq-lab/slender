use crate::{tests::sut::init_pool, *};

#[test]
fn should_return_pause_info() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let prev_pause_info = sut.pool.pause_info();

    sut.pool.set_pause(&false);
    let next_pause_info = sut.pool.pause_info();
    assert!(!next_pause_info.paused);
    assert_eq!(
        prev_pause_info.grace_period_secs,
        next_pause_info.grace_period_secs
    );
    assert_eq!(prev_pause_info.unpaused_at, next_pause_info.unpaused_at);

    sut.pool.set_pause(&true);
    let next_pause_info = sut.pool.pause_info();
    assert!(next_pause_info.paused);
    assert_eq!(
        prev_pause_info.grace_period_secs,
        next_pause_info.grace_period_secs
    );
    assert_eq!(prev_pause_info.unpaused_at, next_pause_info.unpaused_at);
}
