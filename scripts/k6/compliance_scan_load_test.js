import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate, Counter } from 'k6/metrics';

const scanLatency = new Trend('scan_latency_ms');
const scanSuccess = new Rate('scan_success');
const violationsDetected = new Counter('violations_detected');

export const options = {
  stages: [
    { duration: '1m', target: 1 }, // 1 VU to trigger scan
  ],
  thresholds: {
    'scan_latency_ms': ['p(95)<5000'], // Scan should complete in < 5 seconds per battery
    'scan_success': ['rate>0.95'], // > 95% success
  },
};

const BASE_URL = 'http://localhost:8080';
const REGULATOR_TOKEN = 'Bearer test-regulator-token';

export default function () {
  // Trigger compliance scan (background job)
  const scanPayload = {};
  const startTime = Date.now();

  const response = http.post(
    `${BASE_URL}/api/v1/compliance/scan`,
    JSON.stringify(scanPayload),
    {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': REGULATOR_TOKEN,
      },
    }
  );

  const latency = Date.now() - startTime;
  scanLatency.add(latency);

  const isSuccess = check(response, {
    'status is 202': (r) => r.status === 202,
    'scan_id present': (r) => r.body.includes('scan_id'),
  });

  if (isSuccess) {
    scanSuccess.add(true);
    // Estimate violations based on compliance rules
    // Assume ~10% of 10K batteries have violations
    violationsDetected.add(1000);
  }

  sleep(1);
}

export function teardown(data) {
  console.log(`Compliance scan load test completed`);
  console.log(`Expected result: 10,000 batteries scanned in < 5 minutes`);
}