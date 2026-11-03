// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <QJsonArray>
#include <QJsonObject>
#include <QObject>
#include <QStringList>
#include <QUrl>
#include <QVariantList>

class ApiClient;

class AppController final : public QObject
{
    Q_OBJECT

    Q_PROPERTY(QString version READ version CONSTANT)
    Q_PROPERTY(QString qtVersion READ qtVersion CONSTANT)
    Q_PROPERTY(QString backendVersion READ backendVersion NOTIFY backendVersionChanged)
    Q_PROPERTY(QString baseDictDir READ baseDictDir NOTIFY baseDictDirChanged)
    Q_PROPERTY(bool ready READ isReady NOTIFY readyChanged)
    Q_PROPERTY(bool hasDictionaries READ hasDictionaries NOTIFY dictionariesChanged)
    Q_PROPERTY(QVariantList dictionaries READ dictionaries NOTIFY dictionariesChanged)
    Q_PROPERTY(QStringList selectedDictionaryIds READ selectedDictionaryIds NOTIFY selectionChanged)
    Q_PROPERTY(QString currentDictionaryId READ currentDictionaryId NOTIFY selectionChanged)
    Q_PROPERTY(QString dictionaryStatus READ dictionaryStatus NOTIFY selectionChanged)
    Q_PROPERTY(bool multiMode READ multiMode NOTIFY multiModeChanged)
    Q_PROPERTY(QString searchText READ searchText WRITE setSearchText NOTIFY searchTextChanged)
    Q_PROPERTY(QString currentWord READ currentWord NOTIFY currentWordChanged)
    Q_PROPERTY(QVariantList suggestions READ suggestions NOTIFY suggestionsChanged)
    Q_PROPERTY(QVariantList matches READ matches NOTIFY matchesChanged)
    Q_PROPERTY(QVariantList definitions READ definitions NOTIFY definitionsChanged)
    Q_PROPERTY(int currentDefinitionIndex READ currentDefinitionIndex WRITE setCurrentDefinitionIndex
                   NOTIFY currentDefinitionIndexChanged)
    Q_PROPERTY(bool canGoBack READ canGoBack NOTIFY historyChanged)
    Q_PROPERTY(bool canGoForward READ canGoForward NOTIFY historyChanged)
    Q_PROPERTY(int zoom READ zoom NOTIFY zoomChanged)
    Q_PROPERTY(qreal zoomFactor READ zoomFactor NOTIFY zoomChanged)
    Q_PROPERTY(QString statusText READ statusText NOTIFY statusTextChanged)
    Q_PROPERTY(QString errorTitle READ errorTitle NOTIFY errorChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY errorChanged)
    Q_PROPERTY(bool errorVisible READ errorVisible NOTIFY errorChanged)

    Q_PROPERTY(QVariantList notebooks READ notebooks NOTIFY notebooksChanged)
    Q_PROPERTY(QVariantList bookmarks READ bookmarks NOTIFY bookmarksChanged)
    Q_PROPERTY(QString selectedNotebookId READ selectedNotebookId NOTIFY selectedNotebookChanged)
    Q_PROPERTY(bool notebooksBusy READ notebooksBusy NOTIFY notebooksBusyChanged)

    Q_PROPERTY(bool allowSymlinks READ allowSymlinks NOTIFY settingsChanged)
    Q_PROPERTY(int groupLimit READ groupLimit NOTIFY settingsChanged)
    Q_PROPERTY(QString ecdictStatus READ ecdictStatus NOTIFY ecdictStatusChanged)
    Q_PROPERTY(bool ecdictBusy READ ecdictBusy NOTIFY ecdictStatusChanged)
    Q_PROPERTY(bool ecdictFullInstalled READ ecdictFullInstalled NOTIFY ecdictStatusChanged)

    Q_PROPERTY(QString cssDictionaryId READ cssDictionaryId NOTIFY cssStateChanged)
    Q_PROPERTY(QString cssDictionaryName READ cssDictionaryName NOTIFY cssStateChanged)
    Q_PROPERTY(QString cssText READ cssText NOTIFY cssTextChanged)
    Q_PROPERTY(QString cssStatus READ cssStatus NOTIFY cssStatusChanged)
    Q_PROPERTY(bool cssBusy READ cssBusy NOTIFY cssBusyChanged)

public:
    AppController(ApiClient *api, const QString &version, QObject *parent = nullptr);

    QString version() const { return m_version; }
    QString qtVersion() const { return QString::fromLatin1(qVersion()); }
    QString backendVersion() const { return m_backendVersion; }
    QString baseDictDir() const { return m_baseDictDir; }
    bool isReady() const { return m_ready; }
    bool hasDictionaries() const { return !m_dictionaries.isEmpty(); }
    QVariantList dictionaries() const { return m_dictionaryRows; }
    QStringList selectedDictionaryIds() const { return m_selectedDictIds; }
    QString currentDictionaryId() const { return m_currentDictId; }
    QString dictionaryStatus() const;
    bool multiMode() const { return m_multiMode; }
    QString searchText() const { return m_searchText; }
    QString currentWord() const { return m_currentWord; }
    QVariantList suggestions() const { return m_suggestions; }
    QVariantList matches() const { return m_matches; }
    QVariantList definitions() const { return m_definitions; }
    int currentDefinitionIndex() const { return m_currentDefinitionIndex; }
    bool canGoBack() const;
    bool canGoForward() const;
    int zoom() const { return m_zoom; }
    qreal zoomFactor() const { return m_zoom / 100.0; }
    QString statusText() const { return m_statusText; }
    QString errorTitle() const { return m_errorTitle; }
    QString errorText() const { return m_errorText; }
    bool errorVisible() const { return m_errorVisible; }

    QVariantList notebooks() const { return m_notebooks; }
    QVariantList bookmarks() const { return m_bookmarks; }
    QString selectedNotebookId() const { return m_selectedNotebookId; }
    bool notebooksBusy() const { return m_notebooksBusy; }

    bool allowSymlinks() const { return m_allowSymlinks; }
    int groupLimit() const { return m_groupLimit; }
    QString ecdictStatus() const { return m_ecdictStatus; }
    bool ecdictBusy() const { return m_ecdictBusy; }
    bool ecdictFullInstalled() const { return m_ecdictFullInstalled; }

    QString cssDictionaryId() const { return m_cssDictId; }
    QString cssDictionaryName() const { return m_cssDictName; }
    QString cssText() const { return m_cssText; }
    QString cssStatus() const { return m_cssStatus; }
    bool cssBusy() const { return m_cssBusy; }

    Q_INVOKABLE void start();
    Q_INVOKABLE void dismissError();
    void setCurrentDefinitionIndex(int index);
    Q_INVOKABLE void setSearchText(const QString &text);
    Q_INVOKABLE void requestSuggestions(const QString &prefix);
    Q_INVOKABLE void lookup(const QString &word);
    Q_INVOKABLE void useSuggestion(const QString &word);
    Q_INVOKABLE void selectDictionary(const QString &dictId);
    Q_INVOKABLE void setMultiMode(bool enabled);
    Q_INVOKABLE void refreshDictionaries();
    Q_INVOKABLE void rebuildCurrentIndex();
    Q_INVOKABLE void openDictionaryFolder();
    Q_INVOKABLE void reloadViews();

    Q_INVOKABLE void back();
    Q_INVOKABLE void forward();
    Q_INVOKABLE void setZoom(int zoom);
    Q_INVOKABLE void zoomIn();
    Q_INVOKABLE void zoomOut();
    Q_INVOKABLE void resetZoom();

    Q_INVOKABLE void addBookmark();
    Q_INVOKABLE void loadNotebooks();
    Q_INVOKABLE void selectNotebook(const QString &notebookId);
    Q_INVOKABLE void createNotebook(const QString &name);
    Q_INVOKABLE void renameNotebook(const QString &notebookId, const QString &name);
    Q_INVOKABLE void deleteNotebook(const QString &notebookId);
    Q_INVOKABLE void setDefaultNotebook(const QString &notebookId);
    Q_INVOKABLE void removeBookmark(const QString &word, const QString &dictId,
                                    const QString &notebookId);
    Q_INVOKABLE void activateBookmark(const QString &word, const QString &dictId);
    Q_INVOKABLE void exportAnki(const QString &path, const QString &notebookId = QString());
    Q_INVOKABLE void exportCurrentEntry(const QString &path);
    Q_INVOKABLE QString localFile(const QUrl &url) const;

    Q_INVOKABLE void loadSettings();
    Q_INVOKABLE void saveSettings(bool allowSymlinks, int groupLimit);
    Q_INVOKABLE void refreshEcdictStatus();
    Q_INVOKABLE void installEcdict();

    Q_INVOKABLE bool prepareCssEditor();
    Q_INVOKABLE void previewCss(const QString &css);
    Q_INVOKABLE void saveCss(const QString &css);
    Q_INVOKABLE void removeCssOverride();
    Q_INVOKABLE void restoreBaseCss();
    Q_INVOKABLE void finishCssEditor();

signals:
    void readyChanged();
    void backendVersionChanged();
    void baseDictDirChanged();
    void dictionariesChanged();
    void selectionChanged();
    void multiModeChanged();
    void searchTextChanged();
    void currentWordChanged();
    void suggestionsChanged();
    void matchesChanged();
    void definitionsChanged();
    void currentDefinitionIndexChanged();
    void historyChanged();
    void zoomChanged();
    void statusTextChanged();
    void errorChanged();

    void notebooksChanged();
    void bookmarksChanged();
    void selectedNotebookChanged();
    void notebooksBusyChanged();
    void settingsChanged();
    void ecdictStatusChanged();

    void cssStateChanged();
    void cssTextChanged();
    void cssStatusChanged();
    void cssBusyChanged();

    void reloadRequested();
    void applyCssRequested(const QString &css);

private:
    void loadBootstrap();
    void applyBootstrap(const QJsonObject &bootstrap);
    void setReady(bool ready);
    void setStatus(const QString &message);
    void showError(const QString &title, const QString &message);
    void setBackendUnavailable(const QString &message);

    void rebuildDictionaryRows();
    void restoreDictionarySelection(const QString &preferredId);
    void normalizeSelection();
    QString dictionaryName(const QString &dictId) const;
    void savePreferences(const QJsonObject &patch);

    void appendHistory(const QString &word);
    void navigateHistory(int offset);

    void renderLookupBatch(const QString &word, const QJsonArray &results);
    QUrl buildEntryUrl(const QString &dictId, const QJsonObject &entry) const;
    void updateMatches(const QJsonArray &entries);
    void clearLookupState();

    void updateEcdictStatus(const QJsonObject &status);
    void populateNotebooks(const QJsonArray &notebooks);
    void populateBookmarks(const QJsonArray &bookmarks);
    void setNotebooksBusy(bool busy);
    void setCssBusy(bool busy);
    void setCssState(const QString &dictId, const QString &dictName,
                     const QString &text, const QString &status);

    ApiClient *m_api = nullptr;
    QString m_version;
    QString m_backendVersion;
    QString m_staticBaseUrl;
    QString m_baseDictDir;
    QString m_searchText;
    QString m_currentWord;
    QString m_currentDictId;
    QString m_statusText;
    QString m_errorTitle;
    QString m_errorText;
    QString m_selectedNotebookId;
    QString m_ecdictStatus;
    QString m_cssDictId;
    QString m_cssDictName;
    QString m_cssText;
    QString m_cssBaseCss;
    QString m_cssStatus;
    QStringList m_selectedDictIds;
    QStringList m_history;
    QJsonObject m_preferences;
    QJsonArray m_dictionaries;
    QVariantList m_dictionaryRows;
    QVariantList m_suggestions;
    QVariantList m_matches;
    QVariantList m_definitions;
    QVariantList m_notebooks;
    QVariantList m_bookmarks;
    int m_historyPosition = -1;
    int m_zoom = 100;
    int m_groupLimit = 10;
    int m_suggestionRequest = 0;
    int m_lookupRequest = 0;
    int m_currentDefinitionIndex = -1;
    bool m_ready = false;
    bool m_multiMode = false;
    bool m_allowSymlinks = true;
    bool m_errorVisible = false;
    bool m_notebooksBusy = false;
    bool m_ecdictBusy = false;
    bool m_ecdictFullInstalled = false;
    bool m_cssBusy = false;
};
