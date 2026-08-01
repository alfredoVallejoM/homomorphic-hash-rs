//! Private template for nominal generated binary field value objects.

macro_rules! define_binary_field {
    (
        $(#[$metadata:meta])*
        $name:ident,
        limbs = $limbs:ty,
        repr = $repr:ty,
        implementation = $implementation:ty,
        modulus_tail = $modulus_tail:expr,
        catalog = $catalog:path,
        spec = $spec:expr,
        debug_name = $debug_name:literal
    ) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
        #[repr(transparent)]
        pub struct $name($limbs);

        #[cfg(feature = "portable")]
        static PORTABLE_STRATEGY: crate::__private::PortableStrategy<$name> =
            crate::__private::PortableStrategy::new();

        #[cfg(feature = "portable")]
        static KERNEL_CATALOG: crate::kernel::KernelCatalog<$name> =
            $catalog(PORTABLE_STRATEGY.kernels());

        impl $name {
            #[inline]
            pub(crate) const fn from_limbs(limbs: $limbs) -> Self {
                Self(limbs)
            }

            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            pub(crate) const fn into_limbs(self) -> $limbs {
                self.0
            }

            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            pub(crate) const PCLMUL_MODULUS_TAIL: u64 = $modulus_tail;

            fn write_hex(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                for limb in self.0.as_ref().iter().rev() {
                    write!(formatter, "{limb:016x}")?;
                }
                Ok(())
            }
        }

        impl crate::Field for $name {
            const ZERO: Self = Self([0; core::mem::size_of::<$limbs>() / 8]);
            const ONE: Self = {
                let mut limbs = [0; core::mem::size_of::<$limbs>() / 8];
                limbs[0] = 1;
                Self(limbs)
            };

            #[inline]
            fn add(self, rhs: Self) -> Self {
                Self::from_limbs(crate::binary::add_limbs(self.0, rhs.0))
            }

            #[inline]
            fn sub(self, rhs: Self) -> Self {
                crate::Field::add(self, rhs)
            }

            #[inline]
            fn neg(self) -> Self {
                self
            }

            #[inline]
            fn mul(self, rhs: Self) -> Self {
                Self::from_limbs(
                    <$implementation as crate::binary::BinaryFieldImpl>::multiply(self.0, rhs.0),
                )
            }

            #[inline]
            fn is_zero(&self) -> bool {
                crate::binary::limbs_are_zero(&self.0)
            }
        }

        impl crate::Square for $name {
            #[inline]
            fn square(self) -> Self {
                Self::from_limbs(
                    <$implementation as crate::binary::BinaryFieldImpl>::square(self.0),
                )
            }
        }

        impl crate::Invert for $name {
            fn invert(self) -> Option<Self> {
                crate::binary::invert_binary::<
                    Self,
                    { <$implementation as crate::binary::BinaryFieldImpl>::DEGREE },
                >(self)
            }
        }

        impl crate::Pow for $name {}

        impl crate::CanonicalEncoding for $name {
            type Repr = $repr;

            fn from_canonical(repr: &Self::Repr) -> Result<Self, crate::DecodeError> {
                Ok(Self::from_limbs(crate::binary::decode_limbs(repr.as_ref())))
            }

            fn from_canonical_slice(bytes: &[u8]) -> Result<Self, crate::DecodeError> {
                let expected = <$implementation as crate::binary::BinaryFieldImpl>::CANONICAL_BYTES;
                if bytes.len() != expected {
                    return Err(crate::DecodeError::LengthMismatch {
                        expected,
                        actual: bytes.len(),
                    });
                }
                let mut repr = <Self::Repr as Default>::default();
                repr.as_mut().copy_from_slice(bytes);
                Self::from_canonical(&repr)
            }

            fn to_canonical(self) -> Self::Repr {
                let mut repr = <Self::Repr as Default>::default();
                crate::binary::encode_limbs(self.0, repr.as_mut());
                repr
            }
        }

        impl crate::ExtensionField for $name {
            type Base = crate::F2;
            const DEGREE: usize = <$implementation as crate::binary::BinaryFieldImpl>::DEGREE;

            fn frobenius(self, power: usize) -> Self {
                crate::binary::frobenius_binary::<Self, { Self::DEGREE }>(self, power)
            }

            fn trace(self) -> Self::Base {
                crate::binary::trace_binary::<Self, { Self::DEGREE }>(self)
            }

            fn norm(self) -> Self::Base {
                crate::F2::from_bool(!crate::Field::is_zero(&self))
            }
        }

        impl crate::BinaryPolynomialField for $name {
            const MODULUS_DEGREE: usize = <$implementation as crate::binary::BinaryFieldImpl>::DEGREE;

            #[inline]
            fn mul_by_x(self) -> Self {
                Self::from_limbs(
                    <$implementation as crate::binary::BinaryFieldImpl>::mul_by_x(self.0),
                )
            }

            fn from_polynomial_bytes_mod(bytes_le: &[u8]) -> Self {
                let canonical_bytes =
                    <$implementation as crate::binary::BinaryFieldImpl>::CANONICAL_BYTES;
                if bytes_le.len() <= canonical_bytes {
                    let mut repr =
                        <<Self as crate::CanonicalEncoding>::Repr as Default>::default();
                    repr.as_mut()[..bytes_le.len()].copy_from_slice(bytes_le);
                    return <Self as crate::CanonicalEncoding>::from_canonical(&repr)
                        .expect("every full-width binary polynomial is canonical");
                }
                Self::from_limbs(
                    <$implementation as crate::binary::BinaryFieldImpl>::reduce_polynomial_bytes(
                        bytes_le,
                    ),
                )
            }
        }

        impl crate::StaticField for $name {
            fn spec() -> &'static crate::StaticFieldSpec {
                $spec
            }
        }

        #[cfg(feature = "portable")]
        impl crate::kernel::sealed::Sealed for $name {}

        #[cfg(feature = "portable")]
        impl crate::__private::PortableField for $name {
            fn __portable_strategy() -> &'static crate::__private::PortableStrategy<Self> {
                &PORTABLE_STRATEGY
            }

            fn __kernel_catalog() -> crate::kernel::KernelCatalog<Self> {
                KERNEL_CATALOG
            }
        }

        #[cfg(feature = "portable")]
        impl crate::kernel::BuiltinField for $name {
            fn __kernel_catalog() -> &'static crate::kernel::KernelCatalog<Self> {
                &KERNEL_CATALOG
            }
        }

        impl core::ops::Add for $name {
            type Output = Self;

            #[inline]
            fn add(self, rhs: Self) -> Self::Output {
                crate::Field::add(self, rhs)
            }
        }

        impl core::ops::AddAssign for $name {
            #[inline]
            fn add_assign(&mut self, rhs: Self) {
                *self = crate::Field::add(*self, rhs);
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: Self) -> Self::Output {
                crate::Field::sub(self, rhs)
            }
        }

        impl core::ops::SubAssign for $name {
            #[inline]
            fn sub_assign(&mut self, rhs: Self) {
                *self = crate::Field::sub(*self, rhs);
            }
        }

        impl core::ops::Mul for $name {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: Self) -> Self::Output {
                crate::Field::mul(self, rhs)
            }
        }

        impl core::ops::MulAssign for $name {
            #[inline]
            fn mul_assign(&mut self, rhs: Self) {
                *self = crate::Field::mul(*self, rhs);
            }
        }

        impl core::ops::Neg for $name {
            type Output = Self;

            #[inline]
            fn neg(self) -> Self::Output {
                crate::Field::neg(self)
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.write_hex(formatter)
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(concat!($debug_name, "(0x"))?;
                self.write_hex(formatter)?;
                formatter.write_str(")")
            }
        }
    };
}

pub(crate) use define_binary_field;
