#!/usr/bin/env python3
"""16 keep-alive clients x N requests over a mixed set of routes."""
import http.client, sys, threading, time
port, n, label = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
paths = ["/api/v1/books/gen/1", "/api/v1/search?q=light&limit=50", "/api/v1/books/ps/118",
         "/api/v1/stats", "/api/v1/books", "/api/v1/books/john/3/16", "/api/v1/search?q=lord&testament=NT"]
lat, lock = [], threading.Lock()
def worker():
    c = http.client.HTTPConnection("127.0.0.1", port); mine = []
    for i in range(n):
        t = time.perf_counter(); c.request("GET", paths[i % len(paths)]); c.getresponse().read()
        mine.append((time.perf_counter() - t) * 1000)
    with lock: lat.extend(mine)
t = time.perf_counter(); ts = [threading.Thread(target=worker) for _ in range(16)]
[x.start() for x in ts]; [x.join() for x in ts]; el = time.perf_counter() - t
lat.sort()
print(f"{label}: {len(lat)} req in {el:.1f}s = {len(lat)/el:.0f} req/s, p50 {lat[len(lat)//2]:.2f}ms p99 {lat[int(len(lat)*.99)]:.2f}ms")
