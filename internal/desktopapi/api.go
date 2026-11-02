// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

// Package desktopapi exposes the dictionary engine to the Qt desktop client.
// The server binds to loopback only. Every control request needs the random
// token that the backend prints when it starts.
package desktopapi

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"sync"

	"github.com/gin-gonic/gin"
	"github.com/op/go-logging"
	"github.com/terasum/medict/internal/config"
	"github.com/terasum/medict/internal/static/handler"
	"github.com/terasum/medict/pkg/apis"
	"github.com/terasum/medict/pkg/model"
	"github.com/terasum/medict/pkg/service"
	"github.com/terasum/medict/pkg/service/ankiexport"
)

var log = logging.MustGetLogger("desktopapi")

var safeDictID = regexp.MustCompile(`^[A-Za-z0-9_-]+$`)
var inlinedCSSPattern = regexp.MustCompile(`href="data:text/css;base64,([A-Za-z0-9+/=]+)"`)

// Dependencies are the engine objects owned by the desktop backend process.
type Dependencies struct {
	ConfigDir     string
	Version       string
	Token         string
	Config        *config.Config
	Dicts         *service.DictService
	Bookmarks     *service.BookmarkStore
	Controller    *apis.DictsController
	StaticBaseURL func() string
}

type apiServer struct {
	deps    Dependencies
	cssMu   sync.Mutex
	prefsMu sync.Mutex
	dbReady bool
}

// RegisterRoutes adds the Qt control API to the existing Gin engine.
func RegisterRoutes(engine *gin.Engine, deps Dependencies) {
	server := &apiServer{deps: deps, dbReady: deps.Bookmarks != nil}
	group := engine.Group("/api")
	group.Use(server.requireToken)

	group.GET("/bootstrap", server.bootstrap)
	group.GET("/search", server.search)
	group.POST("/dicts/refresh", server.refreshDictionaries)
	group.POST("/dicts/index", server.buildIndex)

	group.GET("/preferences", server.getPreferences)
	group.POST("/preferences", server.savePreferences)

	group.GET("/bookmarks", server.getBookmarks)
	group.POST("/bookmarks/add", server.addBookmark)
	group.POST("/bookmarks/remove", server.removeBookmark)
	group.GET("/bookmarks/snapshot", server.getBookmarkSnapshot)

	group.GET("/notebooks", server.getNotebooks)
	group.POST("/notebooks/create", server.createNotebook)
	group.POST("/notebooks/rename", server.renameNotebook)
	group.POST("/notebooks/delete", server.deleteNotebook)
	group.POST("/notebooks/default", server.setDefaultNotebook)

	group.POST("/anki", server.exportAnki)
	group.GET("/snapshot", server.snapshot)

	group.GET("/css", server.getCSS)
	group.POST("/css", server.saveCSS)

	group.GET("/ecdict/status", server.ecdictStatus)
	group.POST("/ecdict/install", server.installECDICT)
}

// LoadUserCSSOverrides restores per-dictionary CSS overrides at process start.
func LoadUserCSSOverrides(configDir string) error {
	dir := filepath.Join(configDir, "user_css")
	entries, err := os.ReadDir(dir)
	if errors.Is(err, os.ErrNotExist) {
		return nil
	}
	if err != nil {
		return err
	}
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".css") {
			continue
		}
		dictID := strings.TrimSuffix(entry.Name(), ".css")
		if !safeDictID.MatchString(dictID) {
			continue
		}
		data, readErr := os.ReadFile(filepath.Join(dir, entry.Name()))
		if readErr == nil {
			handler.SetUserCSS(dictID, string(data))
		}
	}
	return nil
}

func (a *apiServer) requireToken(c *gin.Context) {
	if c.GetHeader("X-Medict-Token") != a.deps.Token {
		c.AbortWithStatusJSON(http.StatusUnauthorized,
			model.BuildRawError("invalid desktop API token", model.BadParamErrCode))
		return
	}
	c.Next()
}

func (a *apiServer) bootstrap(c *gin.Context) {
	ecdict := any(nil)
	if status, err := a.deps.Dicts.ECDICTStatus(); err == nil {
		ecdict = status
	}
	preferences := map[string]any{}
	if a.deps.Config != nil {
		preferences = a.deps.Config.Preferences()
	}
	baseDir := ""
	if a.deps.Config != nil {
		baseDir = a.deps.Config.BaseDictDir
	}
	c.JSON(http.StatusOK, model.BuildSuccess(gin.H{
		"version":       a.deps.Version,
		"platform":      "desktop",
		"baseDictDir":   baseDir,
		"staticBaseUrl": a.staticBaseURL(),
		"dictionaries":  a.deps.Dicts.Dicts(),
		"preferences":   preferences,
		"ecdict":        ecdict,
	}))
}

func (a *apiServer) staticBaseURL() string {
	if a.deps.StaticBaseURL == nil {
		return ""
	}
	return a.deps.StaticBaseURL()
}

func (a *apiServer) search(c *gin.Context) {
	dictID := c.Query("dict_id")
	word := c.Query("word")
	if dictID == "" || strings.TrimSpace(word) == "" {
		c.JSON(http.StatusOK, model.BuildRawError("dict_id and word are required", model.BadParamErrCode))
		return
	}
	c.JSON(http.StatusOK, a.deps.Controller.SearchWord(dictID, word))
}

func (a *apiServer) refreshDictionaries(c *gin.Context) {
	if err := a.deps.Dicts.InitDicts(); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(a.deps.Dicts.Dicts()))
}

func (a *apiServer) buildIndex(c *gin.Context) {
	var body struct {
		DictID string `json:"dictId"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.DictID == "" {
		c.JSON(http.StatusOK, model.BuildRawError("dictId is required", model.BadParamErrCode))
		return
	}
	c.JSON(http.StatusOK, a.deps.Controller.BuildIndexByDictId(body.DictID))
}

func (a *apiServer) getPreferences(c *gin.Context) {
	if a.deps.Config == nil {
		c.JSON(http.StatusOK, model.BuildSuccess(map[string]any{}))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(a.deps.Config.Preferences()))
}

func (a *apiServer) savePreferences(c *gin.Context) {
	if a.deps.Config == nil {
		c.JSON(http.StatusOK, model.BuildRawError("config is not available", model.InnerSysErrCode))
		return
	}
	var prefs map[string]any
	if err := c.ShouldBindJSON(&prefs); err != nil {
		c.JSON(http.StatusOK, model.BuildRawError("invalid preference payload", model.BadParamErrCode))
		return
	}
	a.prefsMu.Lock()
	defer a.prefsMu.Unlock()
	if err := a.deps.Config.WritePreferences(prefs); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(nil))
}

func (a *apiServer) getBookmarks(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	items, err := a.deps.Bookmarks.All()
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(items))
}

func (a *apiServer) addBookmark(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	var body struct {
		Word       string `json:"word"`
		DictID     string `json:"dictId"`
		NotebookID string `json:"notebookId"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.Word == "" || body.DictID == "" {
		c.JSON(http.StatusOK, model.BuildRawError("word and dictId are required", model.BadParamErrCode))
		return
	}

	dictName := ""
	if dict, ok := a.deps.Dicts.GetDictPlain(body.DictID); ok {
		dictName = dict.Name
	}
	html := ""
	if snapshot, err := a.deps.Controller.RenderSnapshot(body.DictID, body.Word); err == nil {
		html = snapshot
	} else {
		log.Warningf("bookmark snapshot %q failed: %s", body.Word, err)
	}
	if err := a.deps.Bookmarks.Add(body.Word, body.DictID, dictName, body.NotebookID, html); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(nil))
}

func (a *apiServer) removeBookmark(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	var body struct {
		Word       string `json:"word"`
		DictID     string `json:"dictId"`
		NotebookID string `json:"notebookId"`
	}
	if err := c.ShouldBindJSON(&body); err != nil {
		c.JSON(http.StatusOK, model.BuildRawError("invalid bookmark payload", model.BadParamErrCode))
		return
	}
	if err := a.deps.Bookmarks.Remove(body.Word, body.DictID, body.NotebookID); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(nil))
}

func (a *apiServer) getBookmarkSnapshot(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	html, err := a.deps.Bookmarks.GetSnapshot(
		c.Query("word"), c.Query("dictId"), c.Query("notebookId"))
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(html))
}

func (a *apiServer) getNotebooks(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	items, err := a.deps.Bookmarks.Notebooks()
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(items))
}

func (a *apiServer) createNotebook(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	var body struct {
		Name string `json:"name"`
	}
	if err := c.ShouldBindJSON(&body); err != nil {
		c.JSON(http.StatusOK, model.BuildRawError("invalid notebook payload", model.BadParamErrCode))
		return
	}
	notebook, err := a.deps.Bookmarks.AddNotebook(body.Name)
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(notebook))
}

func (a *apiServer) renameNotebook(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	var body struct {
		ID   string `json:"id"`
		Name string `json:"name"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.ID == "" {
		c.JSON(http.StatusOK, model.BuildRawError("id and name are required", model.BadParamErrCode))
		return
	}
	if err := a.deps.Bookmarks.RenameNotebook(body.ID, body.Name); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.BadParamErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(nil))
}

func (a *apiServer) deleteNotebook(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	var body struct {
		ID string `json:"id"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.ID == "" {
		c.JSON(http.StatusOK, model.BuildRawError("id is required", model.BadParamErrCode))
		return
	}
	if err := a.deps.Bookmarks.RemoveNotebook(body.ID); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.BadParamErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(nil))
}

func (a *apiServer) setDefaultNotebook(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	var body struct {
		ID string `json:"id"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.ID == "" {
		c.JSON(http.StatusOK, model.BuildRawError("id is required", model.BadParamErrCode))
		return
	}
	if err := a.deps.Bookmarks.SetDefaultNotebook(body.ID); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.BadParamErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(nil))
}

func (a *apiServer) exportAnki(c *gin.Context) {
	if !a.dbReady {
		c.JSON(http.StatusOK, model.BuildRawError("bookmark store is not available", model.InnerSysErrCode))
		return
	}
	var body struct {
		NotebookID string `json:"notebookId"`
		Path       string `json:"path"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || strings.TrimSpace(body.Path) == "" {
		c.JSON(http.StatusOK, model.BuildRawError("path is required", model.BadParamErrCode))
		return
	}
	rows, err := a.deps.Bookmarks.ExportRows(body.NotebookID)
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	if len(rows) == 0 {
		c.JSON(http.StatusOK, model.BuildRawError("There are no saved words to export.", model.BadParamErrCode))
		return
	}
	exportRows := make([]ankiexport.ExportRow, len(rows))
	for i, row := range rows {
		exportRows[i] = ankiexport.ExportRow{
			Word:         row.Word,
			DictName:     row.DictName,
			NotebookName: row.NotebookName,
			HTML:         row.HTML,
		}
	}
	archive, err := ankiexport.Build(exportRows)
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	if err := os.WriteFile(body.Path, archive, 0o644); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(body.Path))
}

func (a *apiServer) snapshot(c *gin.Context) {
	dictID := c.Query("dictId")
	word := c.Query("word")
	if dictID == "" || word == "" {
		c.JSON(http.StatusOK, model.BuildRawError("dictId and word are required", model.BadParamErrCode))
		return
	}
	html, err := a.deps.Controller.RenderSnapshot(dictID, word)
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	if html == "" {
		c.JSON(http.StatusOK, model.BuildRawError("The entry was not found.", model.BadParamErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(html))
}

func (a *apiServer) getCSS(c *gin.Context) {
	dictID := c.Query("dictId")
	if !safeDictID.MatchString(dictID) {
		c.JSON(http.StatusOK, model.BuildRawError("invalid dictionary id", model.BadParamErrCode))
		return
	}
	saved, err := a.readUserCSS(dictID)
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	base := ""
	if word := strings.TrimSpace(c.Query("word")); word != "" {
		if snapshot, renderErr := a.deps.Controller.RenderSnapshot(dictID, word); renderErr == nil {
			base = extractInlinedCSS(snapshot)
		}
	}
	c.JSON(http.StatusOK, model.BuildSuccess(gin.H{"saved": saved, "base": base}))
}

func (a *apiServer) saveCSS(c *gin.Context) {
	var body struct {
		DictID string `json:"dictId"`
		CSS    string `json:"css"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || !safeDictID.MatchString(body.DictID) {
		c.JSON(http.StatusOK, model.BuildRawError("invalid CSS payload", model.BadParamErrCode))
		return
	}

	a.cssMu.Lock()
	defer a.cssMu.Unlock()

	dir := filepath.Join(a.deps.ConfigDir, "user_css")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	path := filepath.Join(dir, body.DictID+".css")
	if body.CSS == "" {
		if err := os.Remove(path); err != nil && !errors.Is(err, os.ErrNotExist) {
			c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
			return
		}
		handler.SetUserCSS(body.DictID, "")
		c.JSON(http.StatusOK, model.BuildSuccess(nil))
		return
	}
	if err := writeAtomic(path, []byte(body.CSS), 0o644); err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	handler.SetUserCSS(body.DictID, body.CSS)
	c.JSON(http.StatusOK, model.BuildSuccess(nil))
}

func (a *apiServer) readUserCSS(dictID string) (string, error) {
	path := filepath.Join(a.deps.ConfigDir, "user_css", dictID+".css")
	data, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		return "", nil
	}
	return string(data), err
}

func (a *apiServer) ecdictStatus(c *gin.Context) {
	status, err := a.deps.Dicts.ECDICTStatus()
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(status))
}

func (a *apiServer) installECDICT(c *gin.Context) {
	ctx, cancel := context.WithCancel(c.Request.Context())
	defer cancel()
	count, err := a.deps.Dicts.InstallFullECDICT(ctx)
	if err != nil {
		c.JSON(http.StatusOK, model.BuildError(err, model.InnerSysErrCode))
		return
	}
	c.JSON(http.StatusOK, model.BuildSuccess(gin.H{
		"entryCount": count,
		"edition":    "full",
	}))
}

func extractInlinedCSS(snapshot string) string {
	parts := make([]string, 0)
	seen := make(map[string]bool)
	for _, match := range inlinedCSSPattern.FindAllStringSubmatch(snapshot, -1) {
		data, err := base64.StdEncoding.DecodeString(match[1])
		if err != nil || len(data) == 0 || seen[string(data)] {
			continue
		}
		seen[string(data)] = true
		parts = append(parts, string(data))
	}
	return strings.Join(parts, "\n\n")
}

func writeAtomic(path string, data []byte, perm os.FileMode) error {
	tmp, err := os.CreateTemp(filepath.Dir(path), ".medict-qt-css-*")
	if err != nil {
		return err
	}
	tmpPath := tmp.Name()
	defer os.Remove(tmpPath)
	if err := tmp.Chmod(perm); err != nil {
		tmp.Close()
		return err
	}
	if _, err := tmp.Write(data); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Sync(); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	return replaceFile(tmpPath, path)
}

func (a *apiServer) String() string {
	return fmt.Sprintf("desktopapi(%s)", a.deps.Version)
}
