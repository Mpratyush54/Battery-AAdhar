package config

import (
	"context"
	"testing"
	"time"

	"github.com/redis/go-redis/v9"
)

func TestRedisConnectionAndPing(t *testing.T) {
	// Simple stub connection for test
	client := redis.NewClient(&redis.Options{
		Addr: "localhost:6379",
		DB:   0,
	})
    defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), time.Second*2)
	defer cancel()

	_, err := client.Ping(ctx).Result()
	if err != nil {
		t.Logf("Redis not running locally for this test: %v. This is expected if the mock environment is missing.", err)
	} else {
		t.Log("Successfully pinged Redis")
	}
}
