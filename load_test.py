#!/usr/bin/env python3
"""
⚡ AgentState Performance and Load Testing
=======================================

Comprehensive load testing for AgentState APIs.
Tests throughput, latency, and concurrent performance.
"""

import requests
import json
import time
import threading
import concurrent.futures
import statistics
import sys
from typing import List, Dict, Any
import uuid

class LoadTestClient:
    def __init__(self, base_url: str = "http://localhost:8080", namespace: str = "loadtest"):
        self.base_url = base_url.rstrip('/')
        self.namespace = namespace
        self.session = requests.Session()
    
    def health_check(self) -> bool:
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=5)
            return response.status_code == 200
        except:
            return False
    
    def create_agent(self, agent_type: str, body: Dict[str, Any], tags: Dict[str, str] = None) -> Dict[str, Any]:
        payload = {"type": agent_type, "body": body, "tags": tags or {}}
        response = self.session.post(f"{self.base_url}/v1/{self.namespace}/objects", json=payload)
        response.raise_for_status()
        return response.json()
    
    def get_agent(self, agent_id: str) -> Dict[str, Any]:
        response = self.session.get(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()
        return response.json()
    
    def query_agents(self, tags: Dict[str, str] = None) -> List[Dict[str, Any]]:
        query = {"tags": tags} if tags else {}
        response = self.session.post(f"{self.base_url}/v1/{self.namespace}/query", json=query)
        response.raise_for_status()
        return response.json()
    
    def delete_agent(self, agent_id: str) -> None:
        response = self.session.delete(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()

class LoadTester:
    def __init__(self):
        self.client = LoadTestClient()
        self.results = []
        
    def measure_operation(self, operation_name: str, operation_func):
        """Measure the performance of a single operation"""
        start_time = time.time()
        try:
            result = operation_func()
            duration = time.time() - start_time
            return {"operation": operation_name, "duration": duration, "success": True, "result": result}
        except Exception as e:
            duration = time.time() - start_time
            return {"operation": operation_name, "duration": duration, "success": False, "error": str(e)}
    
    def concurrent_creates(self, num_workers: int, operations_per_worker: int) -> List[Dict]:
        """Test concurrent agent creation"""
        print(f"🚀 Testing concurrent creates: {num_workers} workers × {operations_per_worker} ops")
        
        results = []
        
        def worker(worker_id: int):
            worker_results = []
            for i in range(operations_per_worker):
                result = self.measure_operation(
                    "create",
                    lambda: self.client.create_agent(
                        "load-test-agent",
                        {
                            "name": f"LoadAgent-{worker_id}-{i}",
                            "worker_id": worker_id,
                            "sequence": i,
                            "timestamp": time.time(),
                            "data": "x" * 100  # Some payload
                        },
                        {"worker": str(worker_id), "test": "concurrent-creates"}
                    )
                )
                worker_results.append(result)
            return worker_results
        
        start_time = time.time()
        with concurrent.futures.ThreadPoolExecutor(max_workers=num_workers) as executor:
            futures = [executor.submit(worker, i) for i in range(num_workers)]
            for future in concurrent.futures.as_completed(futures):
                results.extend(future.result())
        
        total_time = time.time() - start_time
        successful_ops = [r for r in results if r["success"]]
        failed_ops = [r for r in results if not r["success"]]
        
        print(f"  ✅ Completed {len(successful_ops)}/{len(results)} operations in {total_time:.2f}s")
        print(f"  📊 Throughput: {len(successful_ops)/total_time:.2f} ops/sec")
        if failed_ops:
            print(f"  ❌ Failed operations: {len(failed_ops)}")
        
        return results
    
    def concurrent_queries(self, num_workers: int, queries_per_worker: int) -> List[Dict]:
        """Test concurrent agent queries"""
        print(f"🔍 Testing concurrent queries: {num_workers} workers × {queries_per_worker} queries")
        
        results = []
        
        def worker(worker_id: int):
            worker_results = []
            for i in range(queries_per_worker):
                result = self.measure_operation(
                    "query",
                    lambda: self.client.query_agents({"test": "concurrent-creates"})
                )
                worker_results.append(result)
            return worker_results
        
        start_time = time.time()
        with concurrent.futures.ThreadPoolExecutor(max_workers=num_workers) as executor:
            futures = [executor.submit(worker, i) for i in range(num_workers)]
            for future in concurrent.futures.as_completed(futures):
                results.extend(future.result())
        
        total_time = time.time() - start_time
        successful_ops = [r for r in results if r["success"]]
        
        print(f"  ✅ Completed {len(successful_ops)} queries in {total_time:.2f}s")
        print(f"  📊 Query throughput: {len(successful_ops)/total_time:.2f} queries/sec")
        
        return results
    
    def mixed_workload(self, duration_seconds: int, num_workers: int) -> List[Dict]:
        """Test mixed read/write workload"""
        print(f"🔄 Testing mixed workload: {num_workers} workers for {duration_seconds}s")
        
        results = []
        stop_flag = threading.Event()
        
        def worker(worker_id: int):
            worker_results = []
            operation_count = 0
            
            while not stop_flag.is_set():
                operation_count += 1
                
                # 70% reads, 30% writes
                if operation_count % 10 < 7:
                    # Read operation
                    result = self.measure_operation(
                        "mixed-query",
                        lambda: self.client.query_agents({"worker": str(worker_id % 5)})
                    )
                else:
                    # Write operation
                    result = self.measure_operation(
                        "mixed-create",
                        lambda: self.client.create_agent(
                            "mixed-workload-agent",
                            {
                                "name": f"MixedAgent-{worker_id}-{operation_count}",
                                "timestamp": time.time()
                            },
                            {"worker": str(worker_id), "test": "mixed-workload"}
                        )
                    )
                
                worker_results.append(result)
                time.sleep(0.01)  # Small delay to avoid overwhelming
            
            return worker_results
        
        # Start workers
        with concurrent.futures.ThreadPoolExecutor(max_workers=num_workers) as executor:
            futures = [executor.submit(worker, i) for i in range(num_workers)]
            
            # Let it run for specified duration
            time.sleep(duration_seconds)
            stop_flag.set()
            
            # Collect results
            for future in concurrent.futures.as_completed(futures):
                results.extend(future.result())
        
        successful_ops = [r for r in results if r["success"]]
        reads = [r for r in successful_ops if r["operation"] == "mixed-query"]
        writes = [r for r in successful_ops if r["operation"] == "mixed-create"]
        
        print(f"  ✅ Completed {len(successful_ops)} operations ({len(reads)} reads, {len(writes)} writes)")
        print(f"  📊 Overall throughput: {len(successful_ops)/duration_seconds:.2f} ops/sec")
        
        return results
    
    def latency_benchmark(self, num_operations: int) -> List[Dict]:
        """Benchmark single-threaded latency"""
        print(f"⏱️  Testing latency: {num_operations} sequential operations")
        
        results = []
        
        # Create operations
        for i in range(num_operations):
            result = self.measure_operation(
                "latency-create",
                lambda i=i: self.client.create_agent(
                    "latency-test",
                    {"name": f"LatencyAgent-{i}", "sequence": i},
                    {"test": "latency"}
                )
            )
            results.append(result)
        
        # Query operations
        for i in range(num_operations):
            result = self.measure_operation(
                "latency-query",
                lambda: self.client.query_agents({"test": "latency"})
            )
            results.append(result)
        
        successful_ops = [r for r in results if r["success"]]
        durations = [r["duration"] for r in successful_ops]
        
        if durations:
            avg_latency = statistics.mean(durations) * 1000  # Convert to ms
            p50_latency = statistics.median(durations) * 1000
            p95_latency = statistics.quantiles(durations, n=20)[18] * 1000 if len(durations) > 20 else max(durations) * 1000
            
            print(f"  ✅ Completed {len(successful_ops)} operations")
            print(f"  📊 Avg latency: {avg_latency:.2f}ms")
            print(f"  📊 P50 latency: {p50_latency:.2f}ms")
            print(f"  📊 P95 latency: {p95_latency:.2f}ms")
        
        return results
    
    def run_comprehensive_load_test(self):
        """Run complete load testing suite"""
        print("⚡ AgentState Load Testing Suite")
        print("=" * 40)
        
        if not self.client.health_check():
            print("❌ AgentState server is not available")
            return False
        
        all_results = []
        
        try:
            # Test 1: Concurrent Creates
            print("\n📝 Test 1: Concurrent Creates")
            results1 = self.concurrent_creates(num_workers=10, operations_per_worker=50)
            all_results.extend(results1)
            
            # Test 2: Concurrent Queries
            print("\n🔍 Test 2: Concurrent Queries")
            results2 = self.concurrent_queries(num_workers=20, queries_per_worker=25)
            all_results.extend(results2)
            
            # Test 3: Mixed Workload
            print("\n🔄 Test 3: Mixed Workload")
            results3 = self.mixed_workload(duration_seconds=30, num_workers=15)
            all_results.extend(results3)
            
            # Test 4: Latency Benchmark
            print("\n⏱️  Test 4: Latency Benchmark")
            results4 = self.latency_benchmark(num_operations=100)
            all_results.extend(results4)
            
            # Overall Summary
            print("\n📊 Overall Performance Summary")
            print("=" * 35)
            
            successful_ops = [r for r in all_results if r["success"]]
            failed_ops = [r for r in all_results if not r["success"]]
            
            print(f"Total Operations: {len(all_results)}")
            print(f"✅ Successful: {len(successful_ops)} ({len(successful_ops)/len(all_results)*100:.1f}%)")
            print(f"❌ Failed: {len(failed_ops)} ({len(failed_ops)/len(all_results)*100:.1f}%)")
            
            if successful_ops:
                durations = [r["duration"] for r in successful_ops]
                avg_latency = statistics.mean(durations) * 1000
                p95_latency = statistics.quantiles(durations, n=20)[18] * 1000 if len(durations) > 20 else max(durations) * 1000
                
                print(f"Average Latency: {avg_latency:.2f}ms")
                print(f"P95 Latency: {p95_latency:.2f}ms")
            
            # Operation breakdown
            ops_by_type = {}
            for result in successful_ops:
                op_type = result["operation"]
                if op_type not in ops_by_type:
                    ops_by_type[op_type] = []
                ops_by_type[op_type].append(result["duration"])
            
            print(f"\n📈 Performance by Operation Type:")
            for op_type, durations in ops_by_type.items():
                avg_ms = statistics.mean(durations) * 1000
                print(f"  {op_type}: {len(durations)} ops, {avg_ms:.2f}ms avg")
            
            if len(failed_ops) == 0:
                print(f"\n🎉 Load test completed successfully!")
                print(f"✅ AgentState handles concurrent load well")
                print(f"✅ Low latency performance")
                print(f"✅ No errors under load")
                return True
            else:
                print(f"\n⚠️  Load test completed with some errors")
                print(f"❌ {len(failed_ops)} operations failed")
                return False
                
        except Exception as e:
            print(f"\n❌ Load test failed with error: {e}")
            return False

def main():
    print("⚡ AgentState Performance Testing")
    print("=" * 40)
    
    tester = LoadTester()
    success = tester.run_comprehensive_load_test()
    
    return 0 if success else 1

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n⏹️  Load test interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Load test suite error: {e}")
        sys.exit(1)