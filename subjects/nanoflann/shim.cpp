// The nanoflann exec driver for spatial-bench (day 1.5).
//
// The harness contract (core/src/harness.rs) is the only interface, identical
// for every subject regardless of language:
//   - `--list` prints the registration list (§13) and reads no stdin;
//   - otherwise a RunSpec arrives as JSON on stdin and JSONL points leave on
//     stdout; diagnostics go to stderr.
//
// The engine owns the harness: the seeds come from the spec, the data
// generator is byte-identical to the rust drivers' (chacha8.hpp, self-checked
// at startup), and the timed region is exactly the query loop. nanoflann
// contributes its public API and nothing else.
//
// Dispatch: nanoflann templates on dimensionality and scalar type. The binary
// carries one specialisation per value in the manifest's compile_time_dims
// (via the generated dims.hpp) and both scalars, dispatching at run time on
// the case's tags; k and leaf_max_size are runtime parameters.

#include "chacha8.hpp"
#include "dims.hpp"
#include "json.hpp"

#include <nanoflann.hpp>

#include <algorithm>
#include <array>
#include <chrono>
#include <iostream>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace {

// Anti-optimisation sink: the checksum of every visited index lands here, so
// the query loop cannot be elided. (The rust drivers black-box the same
// checksum.)
volatile std::uint64_t g_sink = 0;

struct Budget {
    double warm_up_ms = 0;
    double measurement_ms = 0;
    std::uint64_t sample_size = 0;
};

const sbjson::Value* tag(const sbjson::Value& tags, const char* key) {
    const sbjson::Value* v = tags.get(key);
    if (v == nullptr) {
        std::fprintf(stderr, "nanoflann driver: case is missing the `%s` tag\n", key);
        std::exit(5);
    }
    return v;
}

std::int64_t tag_int(const sbjson::Value& tags, const char* key) {
    const sbjson::Value* v = tag(tags, key);
    if (v->t != sbjson::Value::T::Int) {
        std::fprintf(stderr, "nanoflann driver: tag `%s` is not an integer\n", key);
        std::exit(5);
    }
    return v->i;
}

std::string tag_word(const sbjson::Value& tags, const char* key) {
    const sbjson::Value* v = tag(tags, key);
    if (v->t != sbjson::Value::T::String) {
        std::fprintf(stderr, "nanoflann driver: tag `%s` is not a string\n", key);
        std::exit(5);
    }
    return v->s;
}

template <typename A, int D>
struct Cloud {
    std::vector<std::array<A, D>> pts;
    std::size_t kdtree_get_point_count() const { return pts.size(); }
    A kdtree_get_pt(std::size_t idx, std::size_t dim) const { return pts[idx][dim]; }
    template <class BBOX>
    bool kdtree_get_bbox(BBOX&) const {
        return false;
    }
};

double ms_since(std::chrono::steady_clock::time_point t) {
    return std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - t)
        .count();
}

/// One measured case: build the index, warm up, sample the query loop, and
/// emit the point as JSON. The stat methodology mirrors the harness contract's
/// shape (mean with a 95% normal-approximation CI, median, mad); the timed
/// region is the query loop — index build stays outside, exactly like the
/// rust drivers.
template <typename A, int D>
std::string run_typed(const sbjson::Value& c, const Budget& budget,
                      std::int64_t tree_size, std::int64_t queries, std::int64_t k,
                      std::int64_t leaf, std::uint64_t point_seed,
                      std::uint64_t query_seed, const std::string& query_kind) {
    auto points = sbgen::ChaCha8::generate<A, D>(
        static_cast<std::uint64_t>(tree_size), point_seed);
    auto probes = sbgen::ChaCha8::generate<A, D>(
        static_cast<std::uint64_t>(queries), query_seed);

    Cloud<A, D> cloud{points};
    using Index = nanoflann::KDTreeSingleIndexAdaptor<
        nanoflann::L2_Simple_Adaptor<A, Cloud<A, D>>, Cloud<A, D>, D>;
    Index index(D, cloud, nanoflann::KDTreeSingleIndexAdaptorParams(
                              static_cast<std::uint32_t>(leaf)));
    index.buildIndex();

    // The timed region: one pass over the whole probe set, checksum as the
    // rust drivers do.
    // nanoflann's IndexType is the index type the tags declare (u64 here);
    // the result set wants matching types.
    using IndexType = std::uint64_t;
    std::vector<IndexType> idx(static_cast<std::size_t>(k));
    std::vector<A> dist(static_cast<std::size_t>(k));
    auto body = [&]() -> std::uint64_t {
        std::uint64_t checksum = 0;
        for (auto& q : probes) {
            nanoflann::KNNResultSet<A> rs(static_cast<std::uint32_t>(k));
            rs.init(idx.data(), dist.data());
            index.findNeighbors(rs, q.data());
            for (auto v : idx) checksum += v;
        }
        g_sink += checksum;
        return checksum;
    };

    auto warm_start = std::chrono::steady_clock::now();
    while (ms_since(warm_start) < budget.warm_up_ms) body();

    std::vector<double> samples;
    samples.reserve(budget.sample_size);
    auto measure_start = std::chrono::steady_clock::now();
    while (samples.size() < budget.sample_size &&
           ms_since(measure_start) < budget.measurement_ms) {
        auto t0 = std::chrono::steady_clock::now();
        body();
        double elapsed_ns =
            std::chrono::duration<double, std::nano>(std::chrono::steady_clock::now() - t0)
                .count();
        samples.push_back(elapsed_ns / static_cast<double>(queries));
    }
    if (samples.empty()) {
        std::fprintf(stderr, "nanoflann driver: measurement produced no samples\n");
        std::exit(5);
    }

    std::sort(samples.begin(), samples.end());
    double n = static_cast<double>(samples.size());
    double mean = 0;
    for (double s : samples) mean += s;
    mean /= n;
    double variance = 0;
    for (double s : samples) variance += (s - mean) * (s - mean);
    variance /= n;
    double sd = std::sqrt(variance);
    double median = samples[samples.size() / 2];
    std::vector<double> abs_dev;
    abs_dev.reserve(samples.size());
    for (double s : samples) abs_dev.push_back(std::fabs(s - median));
    std::sort(abs_dev.begin(), abs_dev.end());
    double mad = abs_dev[abs_dev.size() / 2];
    double half = 1.96 * sd / std::sqrt(n);

    // Tags echo verbatim (the parser kept the original text spans), so the
    // point's identity is byte-faithful to what the engine resolved.
    std::string out = "{\"tags\":{";
    bool first = true;
    for (auto& kv : c.get("tags")->obj) {
        if (!first) out += ',';
        first = false;
        const std::string q = "\"";  // a double quote
        out += q + kv.first + q + ":" + kv.second.raw;
    }
    out += "},\"metrics\":{";
    out += std::format(
        "\"latency_ns\":{{\"point\":{:.10e},\"lower\":{:.10e},\"upper\":{:.10e},"
        "\"unit\":\"ns/query\"}},"
        "\"throughput_qps\":{{\"point\":{:.10e},\"unit\":\"queries/s\"}}",
        mean, mean - half, mean + half, 1e9 / mean);
    out += "},\"stats\":{";
    out += std::format(
        "\"samples\":{},\"ci\":0.95,\"std_dev_ns\":{:.10e},\"median_ns\":{:.10e},"
        "\"mad_ns\":{:.10e}",
        samples.size(), sd, median, mad);
    out += "}}";
    return out;
}

}  // namespace

int main(int argc, char** argv) {
    // The generator self-check runs at startup, always: a shim whose data
    // stream disagrees with the rust drivers refuses — to list or to measure
    // — instead of putting incomparable numbers in the dataset. The rust-side
    // counterpart vectors are pinned in the bencher repo's drift checks.
    {
        auto pts = sbgen::ChaCha8::generate<double, 3>(2, 0x5eed000000000301ull);
        if (std::bit_cast<std::uint64_t>(pts[0][0]) != 0x3fc0efc570511b9cull ||
            std::bit_cast<std::uint64_t>(pts[1][1]) != 0x3fcba6b42d1d3ad4ull) {
            std::fprintf(stderr,
                         "nanoflann driver: the data generator disagrees with the "
                         "rust drivers — refusing to measure (the rand upgrade "
                         "that changed the stream must land everywhere at once)\n");
            return 6;
        }
    }
    if (argc > 1 && std::string(argv[1]) == "--list") {
        // One registration: nanoflann dispatches at run time (axis, k, leaf
        // size) and declares no compile-time tag keys, so one registration
        // serves every case.
        std::printf("{\"compile_time\": []}\n");
        return 0;
    }

    std::string input((std::istreambuf_iterator<char>(std::cin)),
                      std::istreambuf_iterator<char>());
    sbjson::Value spec;
    try {
        spec = sbjson::parse(input);
    } catch (const std::exception& e) {
        std::fprintf(stderr, "nanoflann driver: bad spec: %s\n", e.what());
        return 2;
    }

    const sbjson::Value* version = spec.get("harness_version");
    if (version == nullptr || version->t != sbjson::Value::T::Int ||
        version->i != 1) {
        std::fprintf(stderr, "nanoflann driver: spec is not harness version 1\n");
        return 2;
    }
    const sbjson::Value* budget_v = spec.get("budget");
    const sbjson::Value* cases = spec.get("cases");
    if (budget_v == nullptr || cases == nullptr || cases->t != sbjson::Value::T::Array) {
        std::fprintf(stderr, "nanoflann driver: spec is missing budget or cases\n");
        return 2;
    }
    Budget budget{
        budget_v->get("warm_up_ms") && budget_v->get("warm_up_ms")->t == sbjson::Value::T::Int
            ? static_cast<double>(budget_v->get("warm_up_ms")->i)
            : 0,
        budget_v->get("measurement_ms") &&
                budget_v->get("measurement_ms")->t == sbjson::Value::T::Int
            ? static_cast<double>(budget_v->get("measurement_ms")->i)
            : 0,
        budget_v->get("sample_size") && budget_v->get("sample_size")->t == sbjson::Value::T::Int
            ? static_cast<std::uint64_t>(budget_v->get("sample_size")->i)
            : 0,
    };

    for (const auto& c : cases->arr) {
        const sbjson::Value* tags = c.get("tags");
        if (tags == nullptr) {
            std::fprintf(stderr, "nanoflann driver: case without tags\n");
            return 5;
        }
        std::int64_t tree_size = tag_int(*tags, "tree_size");
        std::int64_t queries = tag_int(*tags, "query_count");
        std::int64_t k = tag_int(*tags, "k");
        std::int64_t dims = tag_int(*tags, "dims");
        std::int64_t leaf = tag_int(*tags, "nanoflann.leaf_max_size");
        std::string axis = tag_word(*tags, "axis");
        std::string query_kind = tag_word(*tags, "query");
        if (query_kind != "exact_nn") {
            std::fprintf(stderr,
                         "nanoflann driver: query kind `%s` is not implemented\n",
                         query_kind.c_str());
            return 5;
        }
        std::uint64_t point_seed = 0;
        std::uint64_t query_seed = 0;
        if (auto* s = c.get("point_seed"); s && s->t == sbjson::Value::T::Int)
            point_seed = static_cast<std::uint64_t>(s->i);
        if (auto* s = c.get("query_seed"); s && s->t == sbjson::Value::T::Int)
            query_seed = static_cast<std::uint64_t>(s->i);

        std::string point;
        bool dispatched = false;
        auto run_with_dims = [&](auto dims_const) {
            constexpr int D = decltype(dims_const)::value;
            if (axis == "f64") {
                point = run_typed<double, D>(c, budget, tree_size, queries, k, leaf,
                                             point_seed, query_seed, query_kind);
                dispatched = true;
            } else if (axis == "f32") {
                point = run_typed<float, D>(c, budget, tree_size, queries, k, leaf,
                                            point_seed, query_seed, query_kind);
                dispatched = true;
            }
        };
        if (!dispatch_dims(static_cast<unsigned>(dims), run_with_dims)) {
            std::fprintf(stderr,
                         "nanoflann driver: no compiled specialisation for dims=%d "
                         "(this binary carries the manifest's compile_time_dims)\n",
                         dims);
            return 3;
        }
        if (!dispatched) {
            std::fprintf(stderr, "nanoflann driver: unknown axis `%s`\n", axis.c_str());
            return 5;
        }
        std::printf("%s\n", point.c_str());
        std::fflush(stdout);
    }
    return 0;
}
