#!/usr/bin/env python3
"""scikit-learn KDTree harness driver."""
import json, subprocess, sys, time
import numpy as np
MASK64=(1<<64)-1
def fail(s): sys.stderr.write(f"sklearn KDTree: {s}\n"); raise SystemExit(5)
def main():
 if '--list' in sys.argv: print('{"compile_time":[]}'); return 0
 from sklearn.neighbors import KDTree
 spec=json.load(sys.stdin)
 if spec.get('harness_version')!=2: fail('unsupported harness')
 for case in spec['cases']:
  t=case['tags']; b=spec['budget']
  if t['parallelism']!='single_threaded': fail('no native parallel query API')
  raw=subprocess.run([case['dataset_generator'],'--kind',case['dataset'],'--dims',str(t['dims']),'--dtype','f64','--tree-count',str(t['tree_size']),'--query-count',str(t['query_count']),'--seed',str(case['random_seed'])],stdout=subprocess.PIPE,check=True).stdout
  n=int.from_bytes(raw[13:21],sys.byteorder); v=np.frombuffer(raw,dtype=np.float64,offset=29); data=np.ascontiguousarray(v[:n*t['dims']].reshape(n,t['dims'])); probes=np.ascontiguousarray(v[n*t['dims']:].reshape(t['query_count'],t['dims']))
  tree=KDTree(data); k=int(t['k'])
  def body():
   checksum=0
   for p in probes: _, ids=tree.query(p.reshape(1,-1),k=k); checksum=(checksum+int(ids[0,-1]))&MASK64
   return checksum
  warm=time.perf_counter_ns()
  while (time.perf_counter_ns()-warm)/1e6<b['warm_up_ms']: body()
  samples=[]; start=time.perf_counter_ns()
  while len(samples)<b['sample_size'] and (time.perf_counter_ns()-start)/1e6<b['measurement_ms']:
   at=time.perf_counter_ns();body();samples.append((time.perf_counter_ns()-at)/len(probes))
  if not samples: fail('no samples')
  samples.sort(); mean=sum(samples)/len(samples); sd=(sum((x-mean)**2 for x in samples)/len(samples))**.5; half=1.96*sd/len(samples)**.5; med=samples[len(samples)//2]
  print(json.dumps({'tags':t,'metrics':{'latency_ns':{'point':mean,'lower':mean-half,'upper':mean+half,'unit':'ns/query'},'throughput_qps':{'point':1e9/mean,'unit':'queries/s'}},'stats':{'samples':len(samples),'ci':.95,'std_dev_ns':sd,'median_ns':med,'mad_ns':sorted(abs(x-med) for x in samples)[len(samples)//2]}}),flush=True)
 return 0
if __name__=='__main__': raise SystemExit(main())
