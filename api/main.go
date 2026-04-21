package main

import (
	"log"
	"net/http"

	"api/config"
	_ "api/docs"
	"api/routes"
)

// @title Battery Pack Aadhaar API
// @version 1.0
// @description The Go Gateway for the BPA Core Engine.
// @host localhost:3000
// @BasePath /
func main() {
	config.InitMicroserviceClient()
	config.InitDB()
	config.InitRedis()

	expressRouter := routes.NewRouter()

	log.Println("API Gateway running on port 3000 (Proxying to Microservice)")
	log.Println("Swagger documentation available at http://localhost:3000/swagger/index.html")
	if err := http.ListenAndServe(":3000", expressRouter); err != nil {
		log.Fatalf("Could not start Express-like server: %v", err)
	}
}
