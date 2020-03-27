use hashbrown::HashSet;

/// https://github.com/avian2/pylog2rotate

pub fn find_indexes_to_delete(indexes: &[u64]) -> Vec<u64> {
    assert!(indexes.iter().all(|&index| index != 0));
    if indexes.len() <= 1 {
        return vec![];
    }

    let n = indexes.iter().max().unwrap();
    let ideal_indexes_to_keep = find_ideal_indexes_to_keep(*n);

    let mut indexes_to_keep = HashSet::new();
    for index in ideal_indexes_to_keep {
        if indexes.contains(&index) {
            indexes_to_keep.insert(index);
        } else {
            let nearest_index = find_nearest_value(&indexes, index);
            indexes_to_keep.insert(nearest_index);
        }
    }

    indexes.into_iter()
        .copied()
        .filter(|index| !indexes_to_keep.contains(index))
        .collect()
}

fn find_nearest_value(elements: &[u64], value: u64) -> u64 {
    let abs_diff = |a: u64, b: u64| if a > b { a - b } else { b - a };

    let mut result = elements[0];
    for &element in elements {
        if abs_diff(element, value) < abs_diff(result, value) {
            result = element;
        }
    }
    result
}

fn find_ideal_indexes_to_keep(mut n: u64) -> Vec<u64> {
    assert!(n >= 1);
    let mut backups = vec![];
    while n > 1 {
        backups.push(n);
        n -= u64::pow(2, f64::log2(n as f64) as u32 - 1);
    }
    backups.push(1);
    backups.reverse();
    backups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main() {
        assert_eq!(find_ideal_indexes_to_keep(1), vec![1]);
        assert_eq!(find_ideal_indexes_to_keep(2), vec![1, 2]);
        assert_eq!(find_ideal_indexes_to_keep(8), vec![1, 2, 4, 8]);
        assert_eq!(find_ideal_indexes_to_keep(10), vec![1, 2, 4, 6, 10]);
        assert_eq!(find_ideal_indexes_to_keep(64), vec![1, 2, 4, 8, 16, 32, 64]);
        assert_eq!(find_ideal_indexes_to_keep(123), vec![1, 2, 3, 5, 7, 11, 19, 27, 43, 59, 91, 123]);
    }
}
