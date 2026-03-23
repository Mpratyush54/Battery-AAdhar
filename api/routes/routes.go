package routes

import (
	"net/http"

	"api/controllers"
	httpSwagger "github.com/swaggo/http-swagger"
)

func SetupRoutes() *http.ServeMux {
	expressRouter := http.NewServeMux()


	expressRouter.HandleFunc("/api/v1/battery/register", func(res http.ResponseWriter, req *http.Request) {
		switch req.Method {
		case http.MethodPost:
			controllers.RegisterBatteryController(res, req)
		default:
			http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		}
	})

	expressRouter.HandleFunc("/api/v1/auth/register", controllers.RegisterStakeholderController)
	expressRouter.HandleFunc("/api/v1/auth/login", controllers.LoginController)
	expressRouter.HandleFunc("/api/v1/auth/refresh", controllers.RefreshController)
	expressRouter.HandleFunc("/api/v1/auth/logout", controllers.LogoutController)

	// Add Swagger HTTP handler
	expressRouter.HandleFunc("/swagger/", httpSwagger.WrapHandler)

	return expressRouter
}
