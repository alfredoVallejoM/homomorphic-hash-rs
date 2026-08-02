//! Private template shared by maintained prime-field value objects.

macro_rules! impl_prime_field_common {
    (
        $name:ident,
        catalog = $catalog:path,
        prime_metadata = $prime_metadata:expr,
        spec = $spec:expr,
        debug_name = $debug_name:literal
    ) => {
        #[cfg(feature = "portable")]
        static PORTABLE_STRATEGY: crate::__private::PortableStrategy<$name> =
            crate::__private::PortableStrategy::new_prime($prime_metadata);

        #[cfg(feature = "portable")]
        static KERNEL_CATALOG: crate::kernel::KernelCatalog<$name> =
            $catalog(PORTABLE_STRATEGY.kernels());

        impl crate::Pow for $name {}

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
                let canonical = crate::CanonicalEncoding::to_canonical(*self);
                for byte in canonical.as_ref().iter().rev() {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(concat!($debug_name, "(0x"))?;
                core::fmt::Display::fmt(self, formatter)?;
                formatter.write_str(")")
            }
        }
    };
}

pub(crate) use impl_prime_field_common;
