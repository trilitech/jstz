use boa_engine::{JsError, JsNativeError, JsResult};

use crate::idl;

use super::{size::SizeAlgorithm, HighWaterMarkAndSizeAlgorithm};

/// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
/// > **highWaterMark, of type `unrestricted double`**
/// >
/// > A non-negative number indicating the high water mark of the stream using this queuing strategy.
///
/// [Streams Standard - § 7.4.][https://streams.spec.whatwg.org/#validate-and-normalize-high-water-mark]
/// > Note: +∞ is explicitly allowed as a valid high water mark. It causes backpressure to never be applied.
pub struct HighWaterMark {
    inner: idl::UnrestrictedDouble,
}

impl TryFrom<idl::UnrestrictedDouble> for HighWaterMark {
    type Error = JsError;

    fn try_from(value: idl::UnrestrictedDouble) -> Result<Self, Self::Error> {
        if value.is_nan() || value < 0.0 {
            return Err(JsNativeError::range().with_message("TODO").into());
        }
        return Ok(HighWaterMark { inner: value });
    }
}

impl HighWaterMark {
    const ZERO: HighWaterMark = HighWaterMark { inner: 0.0 };
    const ONE: HighWaterMark = HighWaterMark { inner: 1.0 };
    const INFINITY: HighWaterMark = HighWaterMark {
        inner: idl::UnrestrictedDouble::INFINITY,
    };
}

/*
// This would be the right default in general, but in the Stream API, this is a priori never used, so adding it is pointless
impl Default for HighWaterMark {
    fn default() -> Self {
        HighWaterMark::INFINITY
    }
}
*/

/// [Streams Standard - § 7.4.][https://streams.spec.whatwg.org/#validate-and-normalize-high-water-mark]
/// > `ExtractHighWaterMark(strategy, defaultHWM)`
pub trait ExtractHighWaterMark {
    fn extract_high_water_mark(
        &self,
        default_hwm: HighWaterMark,
    ) -> JsResult<HighWaterMark>;
}

impl ExtractHighWaterMark for idl::UnrestrictedDouble {
    fn extract_high_water_mark(
        &self,
        default_hwm: HighWaterMark,
    ) -> JsResult<HighWaterMark> {
        HighWaterMark::try_from(*self)
    }
}

impl ExtractHighWaterMark for Option<idl::UnrestrictedDouble> {
    fn extract_high_water_mark(
        &self,
        default_hwm: HighWaterMark,
    ) -> JsResult<HighWaterMark> {
        match self {
            None => Ok(default_hwm),
            Some(value) => value.extract_high_water_mark(default_hwm),
        }
    }
}

impl<T: ExtractHighWaterMark, U> ExtractHighWaterMark
    for HighWaterMarkAndSizeAlgorithm<T, U>
{
    fn extract_high_water_mark(
        &self,
        default_hwm: HighWaterMark,
    ) -> JsResult<HighWaterMark> {
        self.high_water_mark.extract_high_water_mark(default_hwm)
    }
}
