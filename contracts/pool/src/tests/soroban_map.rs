use soroban_sdk::{Env, Map};

#[test]
fn should_sort_() {
    let env = Env::default();

    let mut map: Map<u32, u32> = Map::new(&env);
    let sorted_keys: &[u32; 10] = &[1, 2, 10, 11, 12, 22, 43, 89, 97, 100];
    let sorted_values_by_keys: &[u32; 10] = &[45, 2, 32, 10, 8, 1, 11, 98, 9, 0];

    map.set(sorted_keys[7], sorted_values_by_keys[7]);
    map.set(sorted_keys[5], sorted_values_by_keys[5]);
    map.set(sorted_keys[0], sorted_values_by_keys[0]);
    map.set(sorted_keys[2], sorted_values_by_keys[2]);
    map.set(sorted_keys[8], sorted_values_by_keys[8]);
    map.set(sorted_keys[3], sorted_values_by_keys[3]);
    map.set(sorted_keys[6], sorted_values_by_keys[6]);
    map.set(sorted_keys[4], sorted_values_by_keys[4]);
    map.set(sorted_keys[9], sorted_values_by_keys[9]);
    map.set(sorted_keys[1], sorted_values_by_keys[1]);

    let mut i: usize = 0;
    for (key, _) in map.clone() {
        assert_eq!(key, sorted_keys[i]);
        i += 1;
    }

    i = 0;
    for value in map.values() {
        assert_eq!(value, sorted_values_by_keys[i]);
        i += 1;
    }
}
