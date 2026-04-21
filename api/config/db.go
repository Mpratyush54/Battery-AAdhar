package config

import (
	"context"
	"database/sql"
	"log"
	"os"

	"github.com/infisical/go-sdk"
	"github.com/joho/godotenv"
	_ "github.com/lib/pq"
)

var DB *sql.DB

func InitDB() {
	var err error

	// Load Rust Engine env fallback if it exists (try different depths for test runners)
	_ = godotenv.Load("../core/.env")
	_ = godotenv.Load("../../core/.env")

	connStr := os.Getenv("DATABASE_URL")

	// If DATABASE_URL is not provided directly, try to fetch from Infisical
	if connStr == "" {
		clientID := os.Getenv("INFISICAL_CLIENT_ID")
		clientSecret := os.Getenv("INFISICAL_CLIENT_SECRET")
		projectID := os.Getenv("INFISICAL_PROJECT_ID")
		env := os.Getenv("INFISICAL_ENV")
		
		if env == "" {
			env = "dev"
		}

		if clientID != "" && clientSecret != "" {
			log.Println("🔐 Authenticating with Infisical...")

			client := infisical.NewInfisicalClient(context.Background(), infisical.Config{
				SiteUrl: os.Getenv("INFISICAL_BASE_URL"),
			})

			_, err = client.Auth().UniversalAuthLogin(clientID, clientSecret)
			if err != nil {
				log.Fatalf("❌ Infisical authentication failed: %v", err)
			}
			log.Println("✅ Infisical authentication successful")

			secret, err := client.Secrets().Retrieve(infisical.RetrieveSecretOptions{
				SecretKey:   "DATABASE_URL",
				Environment: env,
				ProjectID:   projectID,
				SecretPath:  "/",
			})
			if err != nil {
				log.Printf("⚠️ Failed to fetch DATABASE_URL from Infisical: %v", err)
			} else {
				log.Println("✅ DATABASE_URL retrieved from Infisical")
				connStr = secret.SecretValue
			}
		}
	}

	if connStr == "" {
		// Default local database URL just to avoid crash on spinup
		log.Println("⚠️ Falling back to default localhost Postgres URI")
		connStr = "postgres://bpa_user:bpa_pass@localhost:5432/bpa_db?sslmode=disable"
	}

	DB, err = sql.Open("postgres", connStr)
	if err != nil {
		log.Fatalf("❌ Failed to bind to Postgres database configuration: %v", err)
	}

	if err = DB.Ping(); err != nil {
		log.Fatalf("❌ Failed to reach Postgres on ping: %v", err)
	}

	log.Println("✅ Connected to Postgres database natively from Go Gateway")
}
