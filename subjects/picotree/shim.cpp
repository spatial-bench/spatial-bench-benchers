// PicoTree C++ harness driver.  The source is header-only and its measured
// search_knn API uses no native worker pool.
#include "../nanoflann/json.hpp"
#include <pico_tree/array_traits.hpp>
#include <pico_tree/kd_tree.hpp>
#include <pico_tree/vector_traits.hpp>
#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <format>
#include <iostream>
#include <string>
#include <vector>
namespace {
struct Budget { double warm, measure; size_t samples; };
const sbjson::Value& get(const sbjson::Value& v,const char* k){auto*p=v.get(k);if(!p)throw std::runtime_error(k);return*p;}
long long integer(const sbjson::Value& v,const char*k){auto&p=get(v,k);if(p.t!=sbjson::Value::T::Int)throw std::runtime_error(k);return p.i;}
std::string word(const sbjson::Value&v,const char*k){auto&p=get(v,k);if(p.t!=sbjson::Value::T::String)throw std::runtime_error(k);return p.s;}
double elapsed(std::chrono::steady_clock::time_point t){return std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-t).count();}
std::vector<char> dataset(const sbjson::Value& c,const sbjson::Value&t){std::string cmd=word(c,"dataset_generator")+" --kind "+word(c,"dataset")+" --dims 3 --dtype f64 --tree-count "+std::to_string(integer(t,"tree_size"))+" --query-count "+std::to_string(integer(t,"query_count"))+" --seed "+std::to_string(integer(c,"random_seed"));FILE*p=popen(cmd.c_str(),"r");if(!p)throw std::runtime_error("generator");std::vector<char>b;char x[65536];size_t n;while((n=fread(x,1,sizeof x,p)))b.insert(b.end(),x,x+n);if(pclose(p)||b.size()<29)throw std::runtime_error("dataset");return b;}
std::string run(const sbjson::Value& c,const Budget& b){auto&t=get(c,"tags");if(word(t,"axis")!="f64"||integer(t,"dims")!=3||word(t,"query")!="exact_nn")throw std::runtime_error("unsupported case");auto raw=dataset(c,t);uint64_t n;memcpy(&n,raw.data()+13,8);size_t q=integer(t,"query_count"),k=integer(t,"k");const double* values=reinterpret_cast<const double*>(raw.data()+29);std::vector<std::array<double,3>> points(n),probes(q);memcpy(points.data(),values,n*sizeof(points[0]));memcpy(probes.data(),values+n*3,q*sizeof(probes[0]));pico_tree::kd_tree tree(std::ref(points),pico_tree::max_leaf_size_t(10));std::vector<typename decltype(tree)::neighbor_type> hits;auto body=[&]{uint64_t sum=0;for(auto&p:probes){tree.search_knn(p,k,hits);for(auto&h:hits)sum+=h.index;}return sum;};auto warm=std::chrono::steady_clock::now();while(elapsed(warm)<b.warm)body();std::vector<double>s;auto start=std::chrono::steady_clock::now();while(s.size()<b.samples&&elapsed(start)<b.measure){auto at=std::chrono::steady_clock::now();body();s.push_back(std::chrono::duration<double,std::nano>(std::chrono::steady_clock::now()-at).count()/q);}std::sort(s.begin(),s.end());double mean=0;for(double x:s)mean+=x;mean/=s.size();double var=0;for(double x:s)var+=(x-mean)*(x-mean);var/=s.size();double sd=sqrt(var),half=1.96*sd/sqrt((double)s.size()),med=s[s.size()/2];std::string out="{\"tags\":{";bool first=true;for(auto&kv:get(c,"tags").obj){if(!first)out+=',';first=false;out+='\"'+kv.first+"\":"+kv.second.raw;}out+=std::format("}},\"metrics\":{{\"latency_ns\":{{\"point\":{},\"lower\":{},\"upper\":{},\"unit\":\"ns/query\"}},\"throughput_qps\":{{\"point\":{},\"unit\":\"queries/s\"}}}},\"stats\":{{\"samples\":{},\"ci\":0.95,\"std_dev_ns\":{},\"median_ns\":{},\"mad_ns\":0}}}}",mean,mean-half,mean+half,1e9/mean,s.size(),sd,med);return out;}
}
int main(int argc,char**argv){if(argc>1&&std::string(argv[1])=="--list"){puts("{\"compile_time\": []}");return 0;}try{std::string in((std::istreambuf_iterator<char>(std::cin)),{});auto spec=sbjson::parse(in);auto&bv=get(spec,"budget");Budget b{(double)integer(bv,"warm_up_ms"),(double)integer(bv,"measurement_ms"),(size_t)integer(bv,"sample_size")};for(auto&c:get(spec,"cases").arr){auto out=run(c,b);puts(out.c_str());}return 0;}catch(const std::exception&e){fprintf(stderr,"picotree: %s\n",e.what());return 5;}}
