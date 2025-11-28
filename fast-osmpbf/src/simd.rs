/// Performs SIMD operations to calculate delta decoding
#[inline]
pub fn delta_decode_i64(input: &[i64], output: &mut [i64], mut last: i64) -> i64 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe {
                return delta_avx2(input, output, last);
            }
        }
        if std::is_x86_feature_detected!("sse2") {
            unsafe {
                return delta_sse2(input, output, last);
            }
        }
    }

    // if not on supported x86 architecture
    for (i, &v) in input.iter().enumerate() {
        last += v;
        output[i] = last;
    }
    last
}

#[target_feature(enable = "avx2")]
unsafe fn delta_avx2(input: &[i64], output: &mut [i64], mut last: i64) -> i64 {
    use std::arch::x86_64::*;

    let mut i = 0;
    let step = 4;

    while i + step <= input.len() {
        // load as __m256i properly
        let raw = unsafe { _mm256_loadu_si256(input.as_ptr().add(i).cast::<__m256i>()) };

        // extract lanes
        let mut buf = [0i64; 4];
        unsafe { _mm256_storeu_si256(buf.as_mut_ptr().cast::<__m256i>(), raw) };

        // prefix sum inside vector
        for lane in 0..4 {
            last += buf[lane];
            buf[lane] = last;
        }

        output[i..i + 4].copy_from_slice(&buf);
        i += 4;
    }

    // scalar tail
    while i < input.len() {
        last += input[i];
        output[i] = last;
        i += 1;
    }

    last
}

#[target_feature(enable = "sse2")]
unsafe fn delta_sse2(input: &[i64], output: &mut [i64], mut last: i64) -> i64 {
    use std::arch::x86_64::*;

    let mut i = 0;
    let step = 2;

    while i + step <= input.len() {
        let raw = unsafe { _mm_loadu_si128(input.as_ptr().add(i) as *const __m128i) };

        let mut buf = [0i64; 2];
        unsafe { _mm_storeu_si128(buf.as_mut_ptr() as *mut __m128i, raw) };

        for lane in 0..2 {
            last += buf[lane];
            buf[lane] = last;
        }

        output[i..i + 2].copy_from_slice(&buf);
        i += 2;
    }

    while i < input.len() {
        last += input[i];
        output[i] = last;
        i += 1;
    }

    last
}
