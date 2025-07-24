"""
Performance monitoring utilities for OptiScaler-GUI
"""
import time
import psutil
import threading
from functools import wraps
from utils.config import config

class PerformanceMonitor:
    """Monitor application performance and resource usage"""
    
    def __init__(self):
        self.metrics = {}
        self.start_time = time.time()
        self.monitoring = False
        self.monitor_thread = None
    
    def start_monitoring(self):
        """Start background performance monitoring"""
        if not self.monitoring:
            self.monitoring = True
            self.monitor_thread = threading.Thread(target=self._monitor_loop, daemon=True)
            self.monitor_thread.start()
    
    def stop_monitoring(self):
        """Stop performance monitoring"""
        self.monitoring = False
    
    def _monitor_loop(self):
        """Background monitoring loop"""
        while self.monitoring:
            try:
                process = psutil.Process()
                memory_info = process.memory_info()
                
                self.metrics.update({
                    'memory_mb': memory_info.rss / (1024 * 1024),
                    'cpu_percent': process.cpu_percent(),
                    'thread_count': process.num_threads(),
                    'uptime': time.time() - self.start_time
                })
                
                time.sleep(5)  # Update every 5 seconds
            except Exception:
                pass
    
    def get_current_metrics(self):
        """Get current performance metrics"""
        return self.metrics.copy()
    
    def timing_decorator(self, operation_name):
        """Decorator to time function execution"""
        def decorator(func):
            @wraps(func)
            def wrapper(*args, **kwargs):
                start_time = time.time()
                try:
                    result = func(*args, **kwargs)
                    success = True
                except Exception as e:
                    result = e
                    success = False
                finally:
                    duration = time.time() - start_time
                    self._record_timing(operation_name, duration, success)
                
                if not success:
                    raise result
                return result
            return wrapper
        return decorator
    
    def _record_timing(self, operation, duration, success):
        """Record timing data for an operation"""
        if 'timings' not in self.metrics:
            self.metrics['timings'] = {}
        
        if operation not in self.metrics['timings']:
            self.metrics['timings'][operation] = {
                'count': 0,
                'total_time': 0,
                'avg_time': 0,
                'max_time': 0,
                'min_time': float('inf'),
                'failures': 0
            }
        
        timing_data = self.metrics['timings'][operation]
        timing_data['count'] += 1
        timing_data['total_time'] += duration
        timing_data['avg_time'] = timing_data['total_time'] / timing_data['count']
        timing_data['max_time'] = max(timing_data['max_time'], duration)
        timing_data['min_time'] = min(timing_data['min_time'], duration)
        
        if not success:
            timing_data['failures'] += 1
    
    def get_performance_report(self):
        """Get a formatted performance report"""
        report = []
        report.append("=== Performance Report ===")
        
        # Current metrics
        if self.metrics:
            report.append(f"Memory Usage: {self.metrics.get('memory_mb', 0):.1f} MB")
            report.append(f"CPU Usage: {self.metrics.get('cpu_percent', 0):.1f}%")
            report.append(f"Thread Count: {self.metrics.get('thread_count', 0)}")
            report.append(f"Uptime: {self.metrics.get('uptime', 0):.1f} seconds")
        
        # Timing data
        if 'timings' in self.metrics:
            report.append("\n--- Operation Timings ---")
            for operation, data in self.metrics['timings'].items():
                success_rate = (data['count'] - data['failures']) / data['count'] * 100
                report.append(f"{operation}:")
                report.append(f"  Count: {data['count']}")
                report.append(f"  Avg Time: {data['avg_time']:.3f}s")
                report.append(f"  Max Time: {data['max_time']:.3f}s")
                report.append(f"  Min Time: {data['min_time']:.3f}s")
                report.append(f"  Success Rate: {success_rate:.1f}%")
        
        return "\n".join(report)

# Global performance monitor instance
performance_monitor = PerformanceMonitor()

# Convenience decorator for timing functions
def timed(operation_name):
    """Decorator to time function execution"""
    return performance_monitor.timing_decorator(operation_name)
