// A minimal JSON parser for the harness contract (day 1.5).
//
// The shim must parse a RunSpec and echo tag values verbatim, with no third-
// party dependencies — so this is a compact recursive-descent parser for the
// whole of JSON, keeping each value's original text span (`raw`) so tag values
// round-trip byte-for-byte into emitted points: a tag that went in as `1048576`
// comes out as `1048576`, one that went in as `"f64"` comes out as `"f64"`.

#pragma once
#include <cstdint>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace sbjson {

struct Value {
    enum class T { Null, Bool, Int, Double, String, Array, Object };
    T t = T::Null;
    bool b = false;
    int64_t i = 0;
    double d = 0;
    std::string s;
    std::vector<Value> arr;
    std::vector<std::pair<std::string, Value>> obj;
    /// The value's original text span — used to echo tag values verbatim.
    std::string raw;

    const Value* get(const char* key) const {
        if (t != T::Object) return nullptr;
        for (auto& kv : obj)
            if (kv.first == key) return &kv.second;
        return nullptr;
    }
};

struct ParseError : std::runtime_error {
    using std::runtime_error::runtime_error;
};

struct Parser {
    const std::string& src;
    size_t i = 0;

    explicit Parser(const std::string& s) : src(s) {}

    [[noreturn]] void fail(const char* what) const {
        throw ParseError(std::string(what) + " at byte " + std::to_string(i));
    }
    void ws() {
        while (i < src.size() &&
               (src[i] == ' ' || src[i] == '\t' || src[i] == '\n' || src[i] == '\r'))
            i++;
    }
    void expect(char c) {
        ws();
        if (i >= src.size() || src[i] != c) fail("expected character");
        i++;
    }

    Value parse() {
        ws();
        size_t start = i;
        Value v = value();
        v.raw = src.substr(start, i - start);
        return v;
    }

    Value value() {
        ws();
        size_t start = i;
        if (i >= src.size()) fail("unexpected end of input");
        char c = src[i];
        Value v;
        switch (c) {
            case '{': {
                v.t = Value::T::Object;
                i++;
                ws();
                if (i < src.size() && src[i] == '}') { i++; }
                else {
                    while (true) {
                        ws();
                        if (i >= src.size() || src[i] != '"') fail("object key");
                        std::string key = str();
                        expect(':');
                        Value child = value();
                        v.obj.emplace_back(std::move(key), std::move(child));
                        ws();
                        if (i < src.size() && src[i] == ',') { i++; continue; }
                        expect('}');
                        break;
                    }
                }
                break;
            }
            case '[': {
                v.t = Value::T::Array;
                i++;
                ws();
                if (i < src.size() && src[i] == ']') { i++; }
                else {
                    while (true) {
                        v.arr.push_back(value());
                        ws();
                        if (i < src.size() && src[i] == ',') { i++; continue; }
                        expect(']');
                        break;
                    }
                }
                break;
            }
            case '"':
                v.t = Value::T::String;
                v.s = str();
                break;
            case 't':
                literal("true");
                v.t = Value::T::Bool;
                v.b = true;
                break;
            case 'f':
                literal("false");
                v.t = Value::T::Bool;
                break;
            case 'n':
                literal("null");
                break;
            default: {
                // Number: integral spellings become Int, everything else
                // Double — the untagged TagValue distinction on the rust side.
                size_t num_start = i;
                if (i < src.size() && (src[i] == '-' || src[i] == '+')) i++;
                bool integral = true;
                while (i < src.size() &&
                       (src[i] >= '0' && src[i] <= '9' || src[i] == '.' ||
                        src[i] == 'e' || src[i] == 'E' || src[i] == '+' ||
                        src[i] == '-')) {
                    if (src[i] == '.' || src[i] == 'e' || src[i] == 'E')
                        integral = false;
                    i++;
                }
                if (i == num_start) fail("a value");
                std::string text = src.substr(num_start, i - num_start);
                if (integral) {
                    v.t = Value::T::Int;
                    v.i = std::strtoll(text.c_str(), nullptr, 10);
                } else {
                    v.t = Value::T::Double;
                    v.d = std::strtod(text.c_str(), nullptr);
                }
                break;
            }
        }
        v.raw = src.substr(start, i - start);
        return v;
    }

    void literal(const char* lit) {
        ws();
        for (const char* p = lit; *p; p++) {
            if (i >= src.size() || src[i] != *p) fail("a literal");
            i++;
        }
    }

    std::string str() {
        expect('"');
        std::string out;
        while (true) {
            if (i >= src.size()) fail("unterminated string");
            char c = src[i++];
            if (c == '"') break;
            if (c != '\\') {
                out += c;
                continue;
            }
            if (i >= src.size()) fail("unterminated escape");
            char e = src[i++];
            switch (e) {
                case '"': out += '"'; break;
                case '\\': out += '\\'; break;
                case '/': out += '/'; break;
                case 'b': out += '\b'; break;
                case 'f': out += '\f'; break;
                case 'n': out += '\n'; break;
                case 'r': out += '\r'; break;
                case 't': out += '\t'; break;
                case 'u': {
                    if (i + 4 > src.size()) fail("\\u escape");
                    unsigned cp = std::stoul(src.substr(i, 4), nullptr, 16);
                    i += 4;
                    // UTF-8 encode; surrogate pairs are not needed for the
                    // contract's payloads but decode the simple case anyway.
                    if (cp < 0x80) {
                        out += static_cast<char>(cp);
                    } else if (cp < 0x800) {
                        out += static_cast<char>(0xC0 | (cp >> 6));
                        out += static_cast<char>(0x80 | (cp & 0x3F));
                    } else {
                        out += static_cast<char>(0xE0 | (cp >> 12));
                        out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
                        out += static_cast<char>(0x80 | (cp & 0x3F));
                    }
                    break;
                }
                default: fail("an escape");
            }
        }
        return out;
    }
};

inline Value parse(const std::string& text) {
    Parser p(text);
    Value v = p.parse();
    p.ws();
    if (p.i != text.size()) p.fail("trailing content");
    return v;
}

}  // namespace sbjson
