use crate::tests::sut::init_pool;
use crate::*;

#[test]
fn should_return_ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    sut.pool.set_ir_params(&ir_params_input);

    let ir_params = sut.pool.ir_params().unwrap();

    assert_eq!(ir_params_input.alpha, ir_params.alpha);
    assert_eq!(ir_params_input.initial_rate, ir_params.initial_rate);
    assert_eq!(ir_params_input.max_rate, ir_params.max_rate);
    assert_eq!(ir_params_input.scaling_coeff, ir_params.scaling_coeff);
}
