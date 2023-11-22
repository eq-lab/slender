use crate::{FixedI128, PERCENTAGE_FACTOR};

mod fixedi128 {

    use super::*;

    #[test]
    fn percent_mul() {
        let percent = 500; // 5%
        let value = 1000;
        assert_eq!(
            FixedI128::from_rational(percent, PERCENTAGE_FACTOR)
                .unwrap()
                .mul_int(value)
                .unwrap(),
            50
        );
    }

    #[test]
    fn into_inner() {
        let fixed = FixedI128::from_inner(100);
        assert_eq!(fixed.into_inner(), 100);
    }

    #[test]
    fn from_inner() {
        let inner = FixedI128::DENOMINATOR;
        assert_eq!(FixedI128::from_inner(inner), FixedI128::ONE);
    }

    #[test]
    fn from_rational() {
        let nom = 1;
        let denom = 2;
        let fixed = FixedI128::from_rational(nom, denom).unwrap();
        assert_eq!(fixed.into_inner(), 500_000_000);
    }

    #[test]
    fn from_percentage() {
        let percentage = 500; //5%
        let fixed = FixedI128::from_percentage(percentage).unwrap();
        let inner: i128 = 500 * FixedI128::DENOMINATOR / i128::from(PERCENTAGE_FACTOR);
        assert_eq!(fixed, FixedI128::from_inner(inner))
    }

    #[test]
    fn from_int() {
        let value = 1;
        assert_eq!(FixedI128::from_int(value).unwrap(), FixedI128::ONE);
    }

    #[test]
    fn mul() {
        let two = FixedI128::from_int(2).unwrap();
        let product = two.checked_mul(two).unwrap();
        assert_eq!(product, FixedI128::from_int(4).unwrap());
        assert_eq!(product.into_inner(), 4 * FixedI128::DENOMINATOR);
    }

    #[test]
    fn div() {
        let four = FixedI128::from_int(4).unwrap();
        let two = FixedI128::from_int(2).unwrap();
        let result = four.checked_div(two).unwrap();

        assert_eq!(result, two);

        let quarter = FixedI128::ONE.checked_div(four).unwrap();
        assert_eq!(quarter, FixedI128::from_rational(1, 4).unwrap());
    }

    #[test]
    fn add() {
        let half = FixedI128::from_rational(1, 2).unwrap();
        let another = FixedI128::from_rational(5, 7).unwrap();

        assert_eq!(
            half.checked_add(another).unwrap(),
            FixedI128::from_rational(17, 14).unwrap()
        )
    }

    #[test]
    fn sub() {
        let quarter = FixedI128::from_rational(1, 4).unwrap();
        let result = FixedI128::ONE.checked_sub(quarter).unwrap();

        assert_eq!(result, FixedI128::from_rational(3, 4).unwrap());
    }

    #[test]
    fn mul_int() {
        let value = 1000;
        let quarter = FixedI128::from_rational(1, 4).unwrap();

        assert_eq!(quarter.mul_int(value).unwrap(), 250);

        let value = i128::MAX;
        assert_eq!(quarter.mul_int(value), None);
    }

    #[test]
    fn recip_mul_int() {
        let value = 1000;
        let fixed = FixedI128::from_rational(7, 8).unwrap();
        // 1000 * 8 / 7 = 8000 / 7 = 1142
        assert_eq!(fixed.recip_mul_int(value).unwrap(), 1142);

        let zero = FixedI128::from_inner(0);
        assert_eq!(zero.recip_mul_int(value), None);
    }

    #[test]
    fn recip_mul_int_ceil() {
        let inner = 1000054757;
        let fixed = FixedI128::from_inner(inner);
        let value = 1;
        assert_eq!(fixed.recip_mul_int_ceil(value).unwrap(), 1);

        let value2 = 2;
        assert_eq!(fixed.recip_mul_int_ceil(value2).unwrap(), 2);

        let zero = FixedI128::from_inner(0);
        assert_eq!(zero.recip_mul_int_ceil(value), None);

        assert_eq!(
            fixed.recip_mul_int_ceil(inner).unwrap(),
            FixedI128::DENOMINATOR
        );

        let zero = 0;
        assert_eq!(fixed.recip_mul_int_ceil(zero).unwrap(), 0);

        assert_eq!(
            fixed.recip_mul_int_ceil(inner + 1).unwrap(),
            FixedI128::DENOMINATOR + 1
        );
        assert_eq!(
            fixed.recip_mul_int_ceil(inner + 2).unwrap(),
            FixedI128::DENOMINATOR + 2
        );
        assert_eq!(
            fixed.recip_mul_int_ceil(inner + 9).unwrap(),
            FixedI128::DENOMINATOR + 9
        );
        assert_eq!(
            fixed.recip_mul_int_ceil(inner + 10).unwrap(),
            FixedI128::DENOMINATOR + 10
        );
    }
}
