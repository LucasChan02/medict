// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net"
	"os"
	"os/signal"
	"syscall"

	"github.com/gin-gonic/gin"
	"github.com/terasum/medict/internal/desktopapi"
	"github.com/terasum/medict/internal/entry"
	"github.com/terasum/medict/internal/utils"
	"github.com/terasum/medict/pkg/backserver"
	"github.com/terasum/medict/pkg/service"
)

// Version is set by the GitHub Actions build.
var Version = "dev"

func main() {
	port := flag.Int("port", 0, "loopback TCP port; 0 selects a free port")
	flag.Parse()

	gin.SetMode(gin.ReleaseMode)
	gin.DefaultWriter = io.Discard
	gin.DefaultErrorWriter = os.Stderr

	conf, err := entry.LoadApp()
	if err != nil {
		fatalf("load configuration: %v", err)
	}
	dicts, err := service.NewDictService(conf)
	if err != nil {
		fatalf("create dictionary service: %v", err)
	}
	if err := dicts.InitDicts(); err != nil {
		fmt.Fprintf(os.Stderr, "medictd: dictionary scan failed: %v\n", err)
	}

	configDir, err := utils.AppConfigDir()
	if err != nil {
		fatalf("locate configuration directory: %v", err)
	}
	bookmarks, err := service.NewBookmarkStore(configDir)
	if err != nil {
		fatalf("open bookmark store: %v", err)
	}
	if err := desktopapi.LoadUserCSSOverrides(configDir); err != nil {
		fmt.Fprintf(os.Stderr, "medictd: load CSS overrides: %v\n", err)
	}

	server, err := backserver.NewStaticServer(conf)
	if err != nil {
		fatalf("create HTTP server: %v", err)
	}
	if err := server.SetUp(dicts); err != nil {
		fatalf("set up HTTP server: %v", err)
	}

	token, err := randomToken()
	if err != nil {
		fatalf("create API token: %v", err)
	}
	desktopapi.RegisterRoutes(server.GinEngine, desktopapi.Dependencies{
		ConfigDir:     configDir,
		Version:       Version,
		Token:         token,
		Config:        conf,
		Dicts:         dicts,
		Bookmarks:     bookmarks,
		Controller:    server.Controller,
		StaticBaseURL: server.StaticServerBaseUrl,
	})

	staticURL, err := server.StartOn(fmt.Sprintf("127.0.0.1:%d", *port))
	if err != nil {
		fatalf("start HTTP server: %v", err)
	}
	host, portText, err := net.SplitHostPort(server.ListenAddr.String())
	if err != nil {
		fatalf("read listening address: %v", err)
	}

	ready, err := json.Marshal(map[string]any{
		"host":      host,
		"port":      portText,
		"url":       staticURL,
		"apiUrl":    fmt.Sprintf("http://%s/api", server.ListenAddr.String()),
		"token":     token,
		"version":   Version,
		"configDir": configDir,
	})
	if err != nil {
		fatalf("encode ready message: %v", err)
	}
	fmt.Printf("MEDICT_READY %s\n", ready)

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)
	stdinClosed := make(chan struct{})
	go func() {
		_, _ = io.Copy(io.Discard, os.Stdin)
		close(stdinClosed)
	}()

	select {
	case <-stop:
	case <-stdinClosed:
	}

	server.GracefulStop()
	_ = dicts.Close()
	_ = bookmarks.Close()
}

func randomToken() (string, error) {
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	return hex.EncodeToString(buf), nil
}

func fatalf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "medictd: "+format+"\n", args...)
	os.Exit(1)
}
