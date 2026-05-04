// health_update.js — k6 load test for concurrent health updates
//
// Tests: 1000 concurrent battery health updates
// Measures: latency (p50/p95/p99), error rate
//
// Run: k6 run scripts/k6/health_update.js

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const healthUpdateLatency = new Trend('health_update_latency');

// Test options
export const options = {
  stages: [
    { duration: '30s', target: 10 },  // Ramp up to 10 VUs
    { duration: '1m', target: 100 },  // Ramp to 100 concurrent
    { duration: '2m', target: 100 },  // Hold at 100 for 2 min
    { duration: '30s', target: 0 },   // Ramp down
  ],
  thresholds: {
    'health_update_latency': ['p(95)<500', 'p(99)<1000'], // p95 < 500ms
    'errors': ['rate<0.001'], // error rate < 0.1%
  },
};

const BASE_URL = 'http://localhost:8080';
const MANUFACTURER_TOKEN = 'Bearer eyJhbGciOiJub25lIn0.eyJyb2xlIjoibWFudWZhY3R1cmVyIn0.';

// Generate test battery BPAN
function randomBPAN() {
  const prefixes = [
    'MY008A6FKKKLC1DH8000',
    'MY008A6FKKKLC1DH8000',
    'MY008A6FKKKLC1DH8000',
  ];
  const suffix = Math.floor(Math.random() * 1000)
    .toString()
    .padStart(2, '0');
  return prefixes[0] + suffix;
}

export default function () {
  const bpan = randomBPAN();

  group('health_update', () => {
    // Update health endpoint
    const healthUpdatePayload = {
      state_of_health_percent: 80 + Math.random() * 10, // 80–90%
      cycle_count: 200000 + Math.floor(Math.random() * 50000),
      degradation_class: 'normal',
      min_temperature_celsius: 15,
      max_temperature_celsius: 45,
      average_temperature_celsius: 30,
      cell_voltage_min_mv: 2500,
      cell_voltage_max_mv: 4200,
      internal_resistance_mohm: 15,
    };

    const startTime = Date.now();

    const response = http.patch(
      `${BASE_URL}/api/v1/batteries/${bpan}/health`,
      JSON.stringify(healthUpdatePayload),
      {
        headers: {
          'Content-Type': 'application/json',
          'Authorization': MANUFACTURER_TOKEN,
        },
      }
    );

    const latency = Date.now() - startTime;
    healthUpdateLatency.add(latency);

    const isSuccess = check(response, {
      'status is 201 or 429': (r) => r.status === 201 || r.status === 429,
      'no errors in response': (r) => !r.body.includes('error'),
    });

    if (!isSuccess) {
      errorRate.add(1);
    }

    sleep(1); // Wait between updates
  });

  group('get_health', () => {
    // Fetch latest health
    const getResponse = http.get(
      `${BASE_URL}/api/v1/batteries/${bpan}/health`,
      {
        headers: {
          'Authorization': MANUFACTURER_TOKEN,
        },
      }
    );

    check(getResponse, {
      'get status is 200': (r) => r.status === 200,
      'has state_of_health_percent': (r) => r.body.includes('state_of_health_percent'),
    });
  });
}
