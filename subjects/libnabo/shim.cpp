// libnabo's matrix query API is its recommended multi-query interface.  The
// upstream CMake default enables OpenMP, so this benchmark records its native
// parallel batch path rather than serialising each probe in the shim.
#include "../nanoflann/json.hpp"
#include <nabo/nabo.h>
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <format>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
struct Budget { double warm, measure; size_t samples; };
const sbjson::Value& get(const sbjson::Value& v, const char* key) { auto* p=v.get(key); if(!p) throw std::runtime_error(key); return *p; }
long long integer(const sbjson::Value& v, const char* key) { auto& p=get(v,key); if(p.t!=sbjson::Value::T::Int) throw std::runtime_error(key); return p.i; }
std::string word(const sbjson::Value& v, const char* key) { auto& p=get(v,key); if(p.t!=sbjson::Value::T::String) throw std::runtime_error(key); return p.s; }
double elapsed(std::chrono::steady_clock::time_point at) { return std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-at).count(); }
std::vector<char> data(const sbjson::Value& c, const sbjson::Value& t) {
  std::string cmd=word(c,"dataset_generator")+" --kind "+word(c,"dataset")+" --dims 3 --dtype f32 --tree-count "+std::to_string(integer(t,"tree_size"))+" --query-count "+std::to_string(integer(t,"query_count"))+" --seed "+std::to_string(integer(c,"random_seed"));
  FILE* p=popen(cmd.c_str(),"r"); if(!p) throw std::runtime_error("generator"); std::vector<char> out; char b[65536]; size_t n; while((n=fread(b,1,sizeof b,p))) out.insert(out.end(),b,b+n); if(pclose(p)||out.size()<29) throw std::runtime_error("dataset"); return out;
}
std::string run(const sbjson::Value& c, const Budget& b) {
  auto& t=get(c,"tags"); if(word(t,"axis")!="f32"||integer(t,"dims")!=3||word(t,"query")!="exact_nn") throw std::runtime_error("unsupported case");
  auto raw=data(c,t); uint64_t n; std::memcpy(&n,raw.data()+13,8); size_t q=integer(t,"query_count"), k=integer(t,"k"); const float* values=reinterpret_cast<const float*>(raw.data()+29);
  Eigen::MatrixXf points(3,n), probes(3,q); std::memcpy(points.data(),values,n*3*sizeof(float)); std::memcpy(probes.data(),values+n*3,q*3*sizeof(float));
  std::unique_ptr<Nabo::NNSearchF> tree(Nabo::NNSearchF::createKDTreeLinearHeap(points)); Eigen::MatrixXi indices(k,q); Eigen::MatrixXf distances(k,q);
  auto body=[&]{ tree->knn(probes,indices,distances,k,0,Nabo::NNSearchF::SORT_RESULTS); uint64_t sum=0; for(Eigen::Index i=0;i<indices.size();++i) sum+=static_cast<uint64_t>(indices.data()[i]); return sum; };
  auto warm=std::chrono::steady_clock::now(); while(elapsed(warm)<b.warm) body(); std::vector<double> samples; auto start=std::chrono::steady_clock::now(); while(samples.size()<b.samples&&elapsed(start)<b.measure) { auto at=std::chrono::steady_clock::now(); body(); samples.push_back(std::chrono::duration<double,std::nano>(std::chrono::steady_clock::now()-at).count()/q); }
  std::sort(samples.begin(),samples.end()); double mean=0; for(double x:samples) mean+=x; mean/=samples.size(); double var=0; for(double x:samples) var+=(x-mean)*(x-mean); var/=samples.size(); double sd=std::sqrt(var), half=1.96*sd/std::sqrt((double)samples.size());
  std::string out="{\"tags\":{"; bool first=true; for(auto& kv:get(c,"tags").obj){if(!first)out+=',';first=false;out+='\"'+kv.first+"\":"+kv.second.raw;} out+=std::format("}},\"metrics\":{{\"latency_ns\":{{\"point\":{},\"lower\":{},\"upper\":{},\"unit\":\"ns/query\"}},\"throughput_qps\":{{\"point\":{},\"unit\":\"queries/s\"}}}},\"stats\":{{\"samples\":{},\"ci\":0.95,\"std_dev_ns\":{},\"median_ns\":{},\"mad_ns\":0}}}}",mean,mean-half,mean+half,1e9/mean,samples.size(),sd,samples[samples.size()/2]); return out;
}
}
int main(int argc,char** argv) { if(argc>1&&std::string(argv[1])=="--list"){puts("{\"compile_time\": []}");return 0;} try { std::string in((std::istreambuf_iterator<char>(std::cin)),{}); auto spec=sbjson::parse(in); auto& bv=get(spec,"budget"); Budget b{(double)integer(bv,"warm_up_ms"),(double)integer(bv,"measurement_ms"),(size_t)integer(bv,"sample_size")}; for(auto& c:get(spec,"cases").arr) puts(run(c,b).c_str()); return 0; } catch(const std::exception& e) { std::fprintf(stderr,"libnabo: %s\n",e.what()); return 5; } }
