// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <QJsonArray>
#include <QJsonObject>
#include <QMainWindow>
#include <QPointer>
#include <QStringList>
#include <QUrl>

class ApiClient;
class CssEditorDialog;
class QAction;
class QCompleter;
class QLabel;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QTabWidget;
class QStringListModel;
class QTimer;

class MainWindow final : public QMainWindow
{
    Q_OBJECT

public:
    MainWindow(ApiClient *api, const QString &version, QWidget *parent = nullptr);

private:
    void buildUi();
    void buildActions();
    void connectSignals();
    void setBackendEnabled(bool enabled);

    void loadBootstrap();
    void applyBootstrap(const QJsonObject &bootstrap);
    void populateDictionaries(const QJsonArray &dictionaries, const QString &preferredId);
    void updateDictionaryStatus();

    QString currentDictId() const;
    QStringList selectedDictIds() const;
    QString dictionaryName(const QString &dictId) const;
    void selectDictionary(const QString &dictId);

    void scheduleSuggestions();
    void requestSuggestions();
    void updateMatches(const QJsonArray &entries);

    void performLookup(const QString &word, bool pushHistory);
    void renderLookupBatch(const QString &word, const QJsonArray &results);
    QUrl buildEntryUrl(const QString &dictId, const QJsonObject &entry) const;
    void showWelcomePage();

    void appendHistory(const QString &word);
    void updateHistoryActions();
    void navigateHistory(int offset);

    void openSettings();
    void openBookmarks();
    void openCssEditor();
    void addBookmark();
    void exportCurrentEntry();
    void exportAnki();
    void refreshDictionaries();
    void rebuildCurrentIndex();
    void openDictionaryFolder();
    void showAbout();

    void setZoom(int zoom);
    void applyUserCss(const QString &css);
    void reloadTabs();
    void savePreferences(const QJsonObject &patch);
    void setStatus(const QString &message);
    void showError(const QString &title, const QString &message);

    ApiClient *m_api = nullptr;
    QString m_version;
    QString m_backendVersion;
    QString m_staticBaseUrl;
    QString m_baseDictDir;
    QString m_currentWord;
    QString m_selectedDictId;
    QJsonObject m_preferences;
    QJsonArray m_dictionaries;
    QStringList m_history;
    int m_historyPosition = -1;
    int m_zoom = 100;
    int m_suggestionRequest = 0;
    int m_lookupRequest = 0;
    bool m_ready = false;
    bool m_restoring = false;

    QLineEdit *m_filterDictionaries = nullptr;
    QLineEdit *m_searchInput = nullptr;
    QListWidget *m_dictionaryList = nullptr;
    QListWidget *m_matches = nullptr;
    QTabWidget *m_definitionTabs = nullptr;
    QCompleter *m_completer = nullptr;
    QStringListModel *m_completionModel = nullptr;
    QLabel *m_statusLabel = nullptr;
    QLabel *m_dictionaryStatusLabel = nullptr;
    QTimer *m_suggestionTimer = nullptr;

    QAction *m_backAction = nullptr;
    QAction *m_forwardAction = nullptr;
    QAction *m_reloadAction = nullptr;
    QAction *m_multiAction = nullptr;
    QAction *m_starAction = nullptr;
    QAction *m_bookmarksAction = nullptr;
    QAction *m_cssAction = nullptr;
    QAction *m_exportAction = nullptr;
    QAction *m_settingsAction = nullptr;
    QAction *m_refreshAction = nullptr;
    QPointer<CssEditorDialog> m_cssEditor;
};
