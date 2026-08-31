// ChaCha8 keyed generator, byte-exact with rand_chacha's ChaCha8Rng
// (rand 0.10) as used by every spatial-bench driver. The harness contract
// promises byte-identical data across subjects, so the C++ shim must consume
// the same stream the rust drivers consume:
//
// - the 32-byte seed comes from rand_core's `seed_from_u64` (a PCG32
//   expansion of the u64, 4 bytes at a time, little-endian);
// - ChaCha8 blocks (RFC 8439's block with 8 rounds = 4 double rounds),
//   64 bytes = 16 u32 words, counter starting at zero, nonce zero;
// - words are consumed in order; `next_u64` assembles two consecutive
//   words little-endian;
// - f64 = (u64 >> 11) * 2^-53 and f32 = (u32 >> 8) * 2^-24, matching rand's
//   StandardUniform; array elements are filled in order.
//
// Verified against the rust drivers' generator vectors in the bencher repo's
// drift checks (day 1.5).

#pragma once
#include <array>
#include <cstdint>
#include <cstring>
#include <vector>

namespace sbgen {

struct ChaCha8 {
    uint32_t key[8];
    uint64_t counter = 0;   // 64-bit block counter, starts at zero
    std::array<uint32_t, 16> buf{};
    size_t index = 16;      // exhausted -> refill on first draw

    static void le32(uint8_t const* p, uint32_t& out) {
        std::memcpy(&out, p, 4);
    }

    // rand_core's `seed_from_u64`: a PCG32 stream fills the 32-byte seed,
    // 4 bytes at a time, little-endian.
    static void seed_words(uint64_t state, uint32_t out[8]) {
        constexpr uint64_t MUL = 6364136223846793005ULL;
        constexpr uint64_t INC = 11634580027462260723ULL;
        uint8_t seed[32];
        for (int chunk = 0; chunk < 8; chunk++) {
            state = state * MUL + INC;
            uint32_t xorshifted =
                static_cast<uint32_t>(((state >> 18) ^ state) >> 27);
            uint32_t rot = static_cast<uint32_t>(state >> 59);
            uint32_t x = (xorshifted >> rot) | (xorshifted << ((32 - rot) & 31));
            std::memcpy(seed + chunk * 4, &x, 4);
        }
        for (int i = 0; i < 8; i++) le32(seed + i * 4, out[i]);
    }

    explicit ChaCha8(uint64_t seed) { seed_words(seed, key); }

    static void quarter(uint32_t& a, uint32_t& b, uint32_t& c, uint32_t& d) {
        a += b; d ^= a; d = (d << 16) | (d >> 16);
        c += d; b ^= c; b = (b << 12) | (b >> 20);
        a += b; d ^= a; d = (d << 8) | (d >> 24);
        c += d; b ^= c; b = (b << 7) | (b >> 25);
    }

    void refill() {
        uint32_t st[16] = {
            0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
            key[0], key[1], key[2], key[3],
            key[4], key[5], key[6], key[7],
            static_cast<uint32_t>(counter),
            static_cast<uint32_t>(counter >> 32),
            0, 0,
        };
        uint32_t w[16];
        std::memcpy(w, st, sizeof(w));
        for (int round = 0; round < 4; round++) {
            quarter(w[0], w[4], w[8], w[12]);
            quarter(w[1], w[5], w[9], w[13]);
            quarter(w[2], w[6], w[10], w[14]);
            quarter(w[3], w[7], w[11], w[15]);
            quarter(w[0], w[5], w[10], w[15]);
            quarter(w[1], w[6], w[11], w[12]);
            quarter(w[2], w[7], w[8], w[13]);
            quarter(w[3], w[4], w[9], w[14]);
        }
        for (int i = 0; i < 16; i++) buf[i] = w[i] + st[i];
        counter++;
        index = 0;
    }

    uint32_t next_u32() {
        if (index >= 16) refill();
        return buf[index++];
    }

    uint64_t next_u64() {
        uint64_t lo = next_u32();
        uint64_t hi = next_u32();
        return (hi << 32) | lo;
    }

    // rand's StandardUniform, matching the rust drivers' point generation.
    template <typename A>
    A standard() {
        if constexpr (std::is_same_v<A, float>) {
            uint32_t x = next_u32();
            return static_cast<float>(x >> 8) * (1.0f / 16777216.0f);
        } else {
            uint64_t x = next_u64();
            return static_cast<double>(x >> 11) * (1.0 / 9007199254740992.0);
        }
    }

    // The harness generator: `generate(count, seed)` — one uniform point per
    // step, elements filled in order (matches the rust drivers' `generate`).
    template <typename A, int D>
    static std::vector<std::array<A, D>> generate(uint64_t count, uint64_t seed) {
        ChaCha8 rng(seed);
        std::vector<std::array<A, D>> out;
        out.reserve(count);
        for (uint64_t i = 0; i < count; i++) {
            std::array<A, D> p;
            for (int d = 0; d < D; d++) p[d] = rng.template standard<A>();
            out.push_back(p);
        }
        return out;
    }
};

}  // namespace sbgen
