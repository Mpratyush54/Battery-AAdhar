// rate_limit.go — Per-battery rate limiting (max 1 update per hour)
//
// Uses Redis to track last update timestamp for each BPAN.

package middleware

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/redis/go-redis/v9"
)

type RateLimiter struct {
	redis *redis.Client
}

func NewRateLimiter(rc *redis.Client) *RateLimiter {
	return &RateLimiter{redis: rc}
}

// HealthUpdateRateLimit checks if a battery can be updated (max 1 per hour)
func (rl *RateLimiter) HealthUpdateRateLimit(bpan string) (allowed bool, err error) {
	key := fmt.Sprintf("health_update:%s", bpan)

	// Try to get existing timestamp
	exists, err := rl.redis.Exists(context.Background(), key).Result()
	if err != nil {
		return false, err
	}

	if exists > 0 {
		// Key exists = update already done in last hour
		return false, nil
	}

	// Set key with 1-hour expiry
	err = rl.redis.Set(context.Background(), key, time.Now().Unix(), 1*time.Hour).Err()
	if err != nil {
		return false, err
	}

	return true, nil
}

// HealthUpdateRateLimitMiddleware HTTP middleware
// Extracts BPAN from chi URL path parameter (e.g. /batteries/{bpan}/health)
func HealthUpdateRateLimitMiddleware(rl *RateLimiter) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Only apply to PATCH/POST (update operations)
			if r.Method != http.MethodPatch && r.Method != http.MethodPost {
				next.ServeHTTP(w, r)
				return
			}

			// Extract BPAN from chi URL path parameter
			bpan := chi.URLParam(r, "bpan")
			if bpan == "" {
				// No BPAN in path — skip rate limiting (let downstream handler decide)
				next.ServeHTTP(w, r)
				return
			}

			// Check rate limit
			allowed, err := rl.HealthUpdateRateLimit(bpan)
			if err != nil {
				http.Error(w, "rate limit check failed", http.StatusInternalServerError)
				return
			}

			if !allowed {
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(http.StatusTooManyRequests) // 429
				json.NewEncoder(w).Encode(map[string]string{
					"error": "max 1 health update per battery per hour",
				})
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
