//! Minimal independent polynomial arithmetic used by Rabin validation.

use crate::spec::identity::hex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BinaryPolynomial {
    words: Vec<u64>,
}

impl BinaryPolynomial {
    pub(crate) fn from_exponents(exponents: &[usize]) -> Self {
        let words = exponents
            .iter()
            .max()
            .map_or(0, |maximum| maximum / u64::BITS as usize + 1);
        let mut polynomial = Self {
            words: vec![0; words],
        };
        for &exponent in exponents {
            polynomial.toggle_bit(exponent);
        }
        polynomial
    }

    pub(crate) fn x() -> Self {
        Self::from_exponents(&[1])
    }

    pub(crate) fn one() -> Self {
        Self::from_exponents(&[0])
    }

    pub(crate) fn is_one(&self) -> bool {
        self == &Self::one()
    }

    pub(crate) fn degree(&self) -> Option<usize> {
        self.words
            .iter()
            .enumerate()
            .rev()
            .find(|(_, word)| **word != 0)
            .map(|(word_index, word)| {
                word_index * u64::BITS as usize + (u64::BITS - 1 - word.leading_zeros()) as usize
            })
    }

    pub(crate) fn xor(&self, rhs: &Self) -> Self {
        let mut result = self.clone();
        result.xor_assign(rhs);
        result
    }

    pub(crate) fn remainder(mut self, modulus: &Self) -> Self {
        let modulus_degree = modulus
            .degree()
            .expect("validation never uses the zero polynomial as modulus");
        while let Some(degree) = self.degree() {
            if degree < modulus_degree {
                break;
            }
            self.xor_shifted_assign(modulus, degree - modulus_degree);
        }
        self
    }

    pub(crate) fn square_mod(&self, modulus: &Self) -> Self {
        let mut squared = Self { words: Vec::new() };
        for (word_index, word) in self.words.iter().copied().enumerate() {
            let mut remaining = word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                squared.toggle_bit(2 * (word_index * u64::BITS as usize + bit));
                remaining &= remaining - 1;
            }
        }
        squared.remainder(modulus)
    }

    pub(crate) fn gcd(mut lhs: Self, mut rhs: Self) -> Self {
        while rhs.degree().is_some() {
            let remainder = lhs.remainder(&rhs);
            lhs = rhs;
            rhs = remainder;
        }
        lhs
    }

    pub(crate) fn to_fixed_hex(&self, bytes: usize) -> String {
        let mut encoded = vec![0_u8; bytes];
        for (word_index, word) in self.words.iter().copied().enumerate() {
            let word_bytes = word.to_le_bytes();
            for (byte_index, byte) in word_bytes.iter().copied().enumerate() {
                let output = word_index * 8 + byte_index;
                if output >= encoded.len() {
                    break;
                }
                encoded[output] = byte;
            }
        }
        hex(&encoded)
    }

    fn toggle_bit(&mut self, bit: usize) {
        let word = bit / u64::BITS as usize;
        if self.words.len() <= word {
            self.words.resize(word + 1, 0);
        }
        self.words[word] ^= 1_u64 << (bit % u64::BITS as usize);
        self.trim();
    }

    fn xor_assign(&mut self, rhs: &Self) {
        if self.words.len() < rhs.words.len() {
            self.words.resize(rhs.words.len(), 0);
        }
        for (lhs, rhs) in self.words.iter_mut().zip(&rhs.words) {
            *lhs ^= rhs;
        }
        self.trim();
    }

    fn xor_shifted_assign(&mut self, rhs: &Self, shift: usize) {
        let word_shift = shift / u64::BITS as usize;
        let bit_shift = shift % u64::BITS as usize;
        let required = rhs.words.len() + word_shift + usize::from(bit_shift != 0);
        if self.words.len() < required {
            self.words.resize(required, 0);
        }
        for (index, word) in rhs.words.iter().copied().enumerate() {
            self.words[index + word_shift] ^= word << bit_shift;
            if bit_shift != 0 {
                self.words[index + word_shift + 1] ^= word >> (u64::BITS as usize - bit_shift);
            }
        }
        self.trim();
    }

    fn trim(&mut self) {
        while self.words.last() == Some(&0) {
            self.words.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BinaryPolynomial;

    #[test]
    fn division_and_gcd_follow_gf2_rules() {
        let modulus = BinaryPolynomial::from_exponents(&[4, 1, 0]);
        let x = BinaryPolynomial::x();
        let mut frobenius = x.clone();
        for _ in 0..4 {
            frobenius = frobenius.square_mod(&modulus);
        }
        assert_eq!(frobenius, x);

        let reducible = BinaryPolynomial::from_exponents(&[4, 2, 0]);
        let candidate = x.square_mod(&reducible).square_mod(&reducible).xor(&x);
        assert!(!BinaryPolynomial::gcd(reducible, candidate).is_one());
    }
}
