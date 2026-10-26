// Package ratelimit implements a token bucket shared across goroutines.
//
// Backtick raw strings hold braces and quotes the scanner must not read as code, and the
// error-check chains are the nesting shape Go code is made of.
package ratelimit

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"sync"
	"time"
)

// ErrExhausted is returned when no token is available within the deadline.
var ErrExhausted = errors.New("ratelimit: bucket exhausted")

// Config describes a bucket.
type Config struct {
	Rate     float64       `json:"rate"`
	Capacity int           `json:"capacity"`
	Burst    time.Duration `json:"burst,omitempty"`
}

// Example is the JSON form of a config, kept as a raw string so the quotes stay literal.
const Example = `{
	"rate": 10,
	"capacity": 100,
	"burst": "1s"
}`

// Bucket is a token bucket safe for concurrent use.
type Bucket struct {
	mu       sync.Mutex
	tokens   float64
	capacity float64
	rate     float64
	updated  time.Time
}

// New builds a bucket from a config, filled to capacity.
func New(config Config) (*Bucket, error) {
	if config.Rate <= 0 {
		return nil, fmt.Errorf("ratelimit: rate must be positive, got %v", config.Rate)
	}

	if config.Capacity <= 0 {
		return nil, fmt.Errorf("ratelimit: capacity must be positive, got %d", config.Capacity)
	}

	return &Bucket{
		tokens:   float64(config.Capacity),
		capacity: float64(config.Capacity),
		rate:     config.Rate,
		updated:  time.Now(),
	}, nil
}

// Parse decodes a config and builds the bucket it describes.
func Parse(data []byte) (*Bucket, error) {
	var config Config

	if err := json.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("ratelimit: decode config: %w", err)
	}

	return New(config)
}

func (b *Bucket) refill(now time.Time) {
	elapsed := now.Sub(b.updated).Seconds()
	b.tokens += elapsed * b.rate

	if b.tokens > b.capacity {
		b.tokens = b.capacity
	}

	b.updated = now
}

// Take removes one token, waiting until one is available or the context ends.
func (b *Bucket) Take(ctx context.Context) error {
	ticker := time.NewTicker(10 * time.Millisecond)
	defer ticker.Stop()

	for {
		b.mu.Lock()
		b.refill(time.Now())

		if b.tokens >= 1 {
			b.tokens--
			b.mu.Unlock()

			return nil
		}

		b.mu.Unlock()

		select {
		case <-ctx.Done():
			return ErrExhausted
		case <-ticker.C:
		}
	}
}

// Drain spends tokens from several workers at once and reports how many succeeded.
func Drain(ctx context.Context, bucket *Bucket, workers int) int {
	var wg sync.WaitGroup
	var mu sync.Mutex
	taken := 0

	for i := 0; i < workers; i++ {
		wg.Add(1)

		go func() {
			defer wg.Done()

			if err := bucket.Take(ctx); err != nil {
				return
			}

			mu.Lock()
			taken++
			mu.Unlock()
		}()
	}

	wg.Wait()

	switch {
	case taken == workers:
		fmt.Println("every worker got a token")
	case taken == 0:
		fmt.Println("no worker got a token")
	default:
		fmt.Printf("%d of %d workers got a token\n", taken, workers)
	}

	return taken
}
