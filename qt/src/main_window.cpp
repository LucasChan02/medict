// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "main_window.h"

#include "api_client.h"
#include "bookmarks_dialog.h"
#include "css_editor_dialog.h"
#include "definition_page.h"
#include "settings_dialog.h"

#include <QAction>
#include <QCompleter>
#include <QDesktopServices>
#include <QDir>
#include <QFileDialog>
#include <QFont>
#include <QIcon>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMap>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QModelIndex>
#include <QSaveFile>
#include <QScrollArea>
#include <QSharedPointer>
#include <QSignalBlocker>
#include <QSizePolicy>
#include <QSplitter>
#include <QStandardPaths>
#include <QStatusBar>
#include <QStyle>
#include <QTabWidget>
#include <QTimer>
#include <QToolBar>
#include <QUrl>
#include <QUrlQuery>
#include <QVBoxLayout>
#include <QVector>
#include <QWebEngineSettings>
#include <QWebEngineView>
#include <QStringListModel>

#include <algorithm>

namespace {

struct LookupItem
{
    QString dictId;
    QString dictName;
    QJsonArray entries;
    QString error;
};

struct LookupBatch
{
    int requestId = 0;
    QString word;
    QVector<LookupItem> items;
    int remaining = 0;
};

QIcon themedIcon(const QString &name, QStyle::StandardPixmap fallback, QWidget *widget)
{
    QIcon icon = QIcon::fromTheme(name);
    if (icon.isNull()) {
        icon = widget->style()->standardIcon(fallback);
    }
    return icon;
}

QString encodedQuery(const QString &path, const QMap<QString, QString> &values)
{
    QUrlQuery query;
    for (auto it = values.cbegin(); it != values.cend(); ++it) {
        query.addQueryItem(it.key(), it.value());
    }
    return path + QLatin1Char('?') + query.query(QUrl::FullyEncoded);
}

QString jsonStringLiteral(const QString &value)
{
    QJsonArray array;
    array.append(value);
    QString encoded = QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact));
    return encoded.mid(1, encoded.size() - 2);
}

} // namespace

MainWindow::MainWindow(ApiClient *api, const QString &version, QWidget *parent)
    : QMainWindow(parent)
    , m_api(api)
    , m_version(version)
    , m_backendVersion(version)
{
    setWindowTitle(tr("Medict"));
    resize(1180, 760);
    setMinimumSize(920, 620);

    buildActions();
    buildUi();
    connectSignals();
    setBackendEnabled(false);
    setStatus(tr("Starting the dictionary engine..."));
    updateHistoryActions();
}

void MainWindow::buildActions()
{
    m_backAction = new QAction(themedIcon(QStringLiteral("go-previous"),
                                          QStyle::SP_ArrowBack, this),
                               tr("Back"), this);
    m_backAction->setShortcut(QKeySequence::Back);
    m_backAction->setToolTip(tr("Go back (Alt+Left)"));

    m_forwardAction = new QAction(themedIcon(QStringLiteral("go-next"),
                                             QStyle::SP_ArrowForward, this),
                                  tr("Forward"), this);
    m_forwardAction->setShortcut(QKeySequence::Forward);
    m_forwardAction->setToolTip(tr("Go forward (Alt+Right)"));

    m_reloadAction = new QAction(themedIcon(QStringLiteral("view-refresh"),
                                            QStyle::SP_BrowserReload, this),
                                 tr("Reload"), this);
    m_reloadAction->setShortcut(QKeySequence::Refresh);
    m_reloadAction->setToolTip(tr("Reload the current entry"));

    m_multiAction = new QAction(themedIcon(QStringLiteral("view-split-left-right"),
                                           QStyle::SP_FileDialogDetailedView, this),
                                tr("Multi"), this);
    m_multiAction->setCheckable(true);
    m_multiAction->setToolTip(tr("Look up the word in all selected dictionaries"));

    m_starAction = new QAction(themedIcon(QStringLiteral("bookmark-new"),
                                          QStyle::SP_DialogApplyButton, this),
                               tr("Save word"), this);
    m_starAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+D")));
    m_starAction->setToolTip(tr("Save the current word to the default notebook"));

    m_bookmarksAction = new QAction(themedIcon(QStringLiteral("view-bookmarks"),
                                               QStyle::SP_FileDialogListView, this),
                                    tr("Notebooks"), this);
    m_bookmarksAction->setToolTip(tr("Open vocabulary notebooks"));

    m_cssAction = new QAction(themedIcon(QStringLiteral("text-x-css"),
                                         QStyle::SP_FileDialogContentsView, this),
                              tr("CSS"), this);
    m_cssAction->setToolTip(tr("Edit a CSS override for the current dictionary"));

    m_exportAction = new QAction(themedIcon(QStringLiteral("document-export"),
                                            QStyle::SP_DialogSaveButton, this),
                                 tr("Export entry"), this);
    m_exportAction->setToolTip(tr("Export the current entry as a self-contained HTML file"));

    m_settingsAction = new QAction(themedIcon(QStringLiteral("preferences-system"),
                                              QStyle::SP_FileDialogListView, this),
                                   tr("Settings"), this);
    m_settingsAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+,")));

    m_refreshAction = new QAction(themedIcon(QStringLiteral("view-refresh"),
                                             QStyle::SP_BrowserReload, this),
                                  tr("Refresh dictionaries"), this);
}

void MainWindow::buildUi()
{
    auto *toolbar = addToolBar(tr("Main"));
    toolbar->setObjectName(QStringLiteral("MainToolbar"));
    toolbar->setMovable(false);
    toolbar->setIconSize(QSize(20, 20));
    toolbar->addAction(m_backAction);
    toolbar->addAction(m_forwardAction);
    toolbar->addAction(m_reloadAction);
    toolbar->addSeparator();

    m_searchInput = new QLineEdit(toolbar);
    m_searchInput->setObjectName(QStringLiteral("searchInput"));
    m_searchInput->setPlaceholderText(tr("Search a word"));
    m_searchInput->setClearButtonEnabled(true);
    m_searchInput->setMinimumWidth(360);
    m_searchInput->setMaximumWidth(760);
    m_searchInput->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    toolbar->addWidget(m_searchInput);

    m_completionModel = new QStringListModel(this);
    m_completer = new QCompleter(m_completionModel, this);
    m_completer->setCaseSensitivity(Qt::CaseInsensitive);
    m_completer->setCompletionMode(QCompleter::PopupCompletion);
    m_completer->setMaxVisibleItems(12);
    m_searchInput->setCompleter(m_completer);

    toolbar->addAction(m_multiAction);
    toolbar->addAction(m_starAction);
    toolbar->addAction(m_bookmarksAction);
    toolbar->addAction(m_cssAction);
    toolbar->addAction(m_exportAction);
    toolbar->addAction(m_settingsAction);

    auto *sidebar = new QWidget(this);
    auto *sidebarLayout = new QVBoxLayout(sidebar);
    sidebarLayout->setContentsMargins(8, 8, 8, 8);
    sidebarLayout->setSpacing(6);

    auto *dictionaryTitle = new QLabel(tr("Dictionaries"), sidebar);
    QFont sectionFont = dictionaryTitle->font();
    sectionFont.setBold(true);
    dictionaryTitle->setFont(sectionFont);
    sidebarLayout->addWidget(dictionaryTitle);

    m_filterDictionaries = new QLineEdit(sidebar);
    m_filterDictionaries->setPlaceholderText(tr("Filter dictionaries"));
    m_filterDictionaries->setClearButtonEnabled(true);
    sidebarLayout->addWidget(m_filterDictionaries);

    m_dictionaryList = new QListWidget(sidebar);
    m_dictionaryList->setSelectionMode(QAbstractItemView::SingleSelection);
    m_dictionaryList->setUniformItemSizes(true);
    sidebarLayout->addWidget(m_dictionaryList, 3);

    auto *matchesTitle = new QLabel(tr("Matches"), sidebar);
    matchesTitle->setFont(sectionFont);
    sidebarLayout->addWidget(matchesTitle);

    m_matches = new QListWidget(sidebar);
    m_matches->setSelectionMode(QAbstractItemView::SingleSelection);
    m_matches->setUniformItemSizes(true);
    sidebarLayout->addWidget(m_matches, 2);

    m_definitionTabs = new QTabWidget(this);
    m_definitionTabs->setDocumentMode(true);
    m_definitionTabs->setTabsClosable(false);
    m_definitionTabs->setMovable(false);

    auto *splitter = new QSplitter(Qt::Horizontal, this);
    splitter->addWidget(sidebar);
    splitter->addWidget(m_definitionTabs);
    splitter->setChildrenCollapsible(false);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({280, 900});
    setCentralWidget(splitter);

    m_statusLabel = new QLabel(tr("Ready"), this);
    m_dictionaryStatusLabel = new QLabel(tr("No dictionaries"), this);
    statusBar()->addWidget(m_statusLabel, 1);
    statusBar()->addPermanentWidget(m_dictionaryStatusLabel);

    auto *fileMenu = menuBar()->addMenu(tr("&File"));
    auto *openFolderAction = fileMenu->addAction(tr("Open dictionary folder"));
    fileMenu->addAction(m_refreshAction);
    fileMenu->addAction(m_settingsAction);
    fileMenu->addAction(m_bookmarksAction);
    fileMenu->addSeparator();
    fileMenu->addAction(m_exportAction);
    auto *exportAnkiAction = fileMenu->addAction(tr("Export Anki package..."));
    fileMenu->addSeparator();
    auto *quitAction = fileMenu->addAction(tr("Quit"));
    quitAction->setShortcut(QKeySequence::Quit);

    auto *viewMenu = menuBar()->addMenu(tr("&View"));
    viewMenu->addAction(m_backAction);
    viewMenu->addAction(m_forwardAction);
    viewMenu->addAction(m_reloadAction);
    viewMenu->addSeparator();
    auto *zoomInAction = viewMenu->addAction(tr("Zoom in"));
    zoomInAction->setShortcut(QKeySequence::ZoomIn);
    auto *zoomOutAction = viewMenu->addAction(tr("Zoom out"));
    zoomOutAction->setShortcut(QKeySequence::ZoomOut);
    auto *resetZoomAction = viewMenu->addAction(tr("Reset zoom"));
    resetZoomAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+0")));
    viewMenu->addSeparator();
    viewMenu->addAction(m_multiAction);

    auto *dictionaryMenu = menuBar()->addMenu(tr("&Dictionaries"));
    auto *rebuildAction = dictionaryMenu->addAction(tr("Rebuild current index"));

    auto *helpMenu = menuBar()->addMenu(tr("&Help"));
    auto *aboutAction = helpMenu->addAction(tr("About Medict"));
    auto *aboutQtAction = helpMenu->addAction(tr("About Qt"));

    connect(openFolderAction, &QAction::triggered, this, &MainWindow::openDictionaryFolder);
    connect(exportAnkiAction, &QAction::triggered, this, &MainWindow::exportAnki);
    connect(quitAction, &QAction::triggered, this, &QWidget::close);
    connect(zoomInAction, &QAction::triggered, this, [this]() { setZoom(m_zoom + 10); });
    connect(zoomOutAction, &QAction::triggered, this, [this]() { setZoom(m_zoom - 10); });
    connect(resetZoomAction, &QAction::triggered, this, [this]() { setZoom(100); });
    connect(rebuildAction, &QAction::triggered, this, &MainWindow::rebuildCurrentIndex);
    connect(aboutAction, &QAction::triggered, this, &MainWindow::showAbout);
    connect(aboutQtAction, &QAction::triggered, this, []() {
        QMessageBox::aboutQt(nullptr, tr("About Qt"));
    });

    showWelcomePage();
    setWindowTitle(tr("Medict %1").arg(m_version));
}

void MainWindow::connectSignals()
{
    connect(m_api, &ApiClient::ready, this, &MainWindow::loadBootstrap);
    connect(m_api, &ApiClient::failed, this, [this](const QString &message) {
        showError(tr("Dictionary engine error"), message);
        setStatus(tr("The dictionary engine is not available."));
    });
    connect(m_api, &ApiClient::backendStopped, this, [this](const QString &message) {
        setStatus(message);
    });

    connect(m_searchInput, &QLineEdit::returnPressed, this, [this]() {
        performLookup(m_searchInput->text(), true);
    });
    connect(m_searchInput, &QLineEdit::textEdited,
            this, [this]() { scheduleSuggestions(); });
    connect(m_completer, QOverload<const QModelIndex &>::of(&QCompleter::activated),
            this, [this](const QModelIndex &index) {
        const QString word = index.data(Qt::DisplayRole).toString();
        if (word.isEmpty()) {
            return;
        }
        m_searchInput->setText(word);
        performLookup(word, true);
    });

    m_suggestionTimer = new QTimer(this);
    m_suggestionTimer->setSingleShot(true);
    m_suggestionTimer->setInterval(250);
    connect(m_suggestionTimer, &QTimer::timeout, this, &MainWindow::requestSuggestions);

    connect(m_dictionaryList, &QListWidget::itemSelectionChanged,
            this, [this]() {
        if (!m_ready || m_restoring) {
            return;
        }
        const QStringList ids = selectedDictIds();
        if (ids.isEmpty()) {
            return;
        }
        m_selectedDictId = m_dictionaryList->currentItem()
            ? m_dictionaryList->currentItem()->data(Qt::UserRole).toString()
            : ids.first();
        updateDictionaryStatus();

        QJsonObject patch;
        if (m_multiAction->isChecked()) {
            QJsonArray active;
            for (const QString &id : ids) {
                active.append(id);
            }
            patch.insert(QStringLiteral("multidictids"), active);
        } else {
            patch.insert(QStringLiteral("selecteddictid"), m_selectedDictId);
        }
        savePreferences(patch);
        if (!m_currentWord.isEmpty()) {
            performLookup(m_currentWord, false);
        }
    });

    connect(m_filterDictionaries, &QLineEdit::textChanged,
            this, [this](const QString &text) {
        const QString needle = text.trimmed();
        for (int row = 0; row < m_dictionaryList->count(); ++row) {
            QListWidgetItem *item = m_dictionaryList->item(row);
            item->setHidden(!needle.isEmpty()
                && !item->text().contains(needle, Qt::CaseInsensitive));
        }
    });

    connect(m_matches, &QListWidget::itemDoubleClicked,
            this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }
        const QByteArray raw = item->data(Qt::UserRole + 1).toByteArray();
        const QJsonDocument document = QJsonDocument::fromJson(raw);
        const QString word = document.isObject()
            ? document.object().value(QStringLiteral("keyword")).toString()
            : item->text();
        if (!word.isEmpty()) {
            m_searchInput->setText(word);
            performLookup(word, true);
        }
    });

    connect(m_multiAction, &QAction::toggled, this, [this](bool enabled) {
        if (m_restoring) {
            return;
        }
        m_dictionaryList->setSelectionMode(enabled
            ? QAbstractItemView::ExtendedSelection
            : QAbstractItemView::SingleSelection);
        if (!enabled && m_dictionaryList->currentItem()) {
            m_dictionaryList->clearSelection();
            m_dictionaryList->currentItem()->setSelected(true);
        }
        updateDictionaryStatus();

        QJsonObject patch;
        patch.insert(QStringLiteral("multimode"), enabled);
        QJsonArray active;
        for (const QString &id : selectedDictIds()) {
            active.append(id);
        }
        patch.insert(QStringLiteral("multidictids"), active);
        savePreferences(patch);
        if (!m_currentWord.isEmpty()) {
            performLookup(m_currentWord, false);
        }
    });

    connect(m_backAction, &QAction::triggered, this, [this]() { navigateHistory(-1); });
    connect(m_forwardAction, &QAction::triggered, this, [this]() { navigateHistory(1); });
    connect(m_reloadAction, &QAction::triggered, this, &MainWindow::reloadTabs);
    connect(m_starAction, &QAction::triggered, this, &MainWindow::addBookmark);
    connect(m_bookmarksAction, &QAction::triggered, this, &MainWindow::openBookmarks);
    connect(m_cssAction, &QAction::triggered, this, &MainWindow::openCssEditor);
    connect(m_exportAction, &QAction::triggered, this, &MainWindow::exportCurrentEntry);
    connect(m_settingsAction, &QAction::triggered, this, &MainWindow::openSettings);
    connect(m_refreshAction, &QAction::triggered, this, &MainWindow::refreshDictionaries);
}

void MainWindow::setBackendEnabled(bool enabled)
{
    m_searchInput->setEnabled(enabled);
    m_dictionaryList->setEnabled(enabled);
    m_matches->setEnabled(enabled);
    m_filterDictionaries->setEnabled(enabled);
    m_reloadAction->setEnabled(enabled);
    m_multiAction->setEnabled(enabled);
    m_starAction->setEnabled(enabled);
    m_bookmarksAction->setEnabled(enabled);
    m_cssAction->setEnabled(enabled);
    m_exportAction->setEnabled(enabled);
    m_settingsAction->setEnabled(enabled);
    m_refreshAction->setEnabled(enabled);
}

void MainWindow::loadBootstrap()
{
    setStatus(tr("Loading dictionaries..."));
    m_api->get(QStringLiteral("/bootstrap"), [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isObject()) {
            showError(tr("Startup error"),
                      error.isEmpty() ? tr("The backend bootstrap response is invalid.") : error);
            return;
        }
        applyBootstrap(data.toObject());
    });
}

void MainWindow::applyBootstrap(const QJsonObject &bootstrap)
{
    m_backendVersion = bootstrap.value(QStringLiteral("version")).toString(m_version);
    m_staticBaseUrl = bootstrap.value(QStringLiteral("staticBaseUrl")).toString();
    if (m_staticBaseUrl.isEmpty()) {
        m_staticBaseUrl = m_api->staticBaseUrl();
    }
    m_baseDictDir = bootstrap.value(QStringLiteral("baseDictDir")).toString();
    m_preferences = bootstrap.value(QStringLiteral("preferences")).toObject();
    m_dictionaries = bootstrap.value(QStringLiteral("dictionaries")).toArray();

    const int savedZoom = m_preferences.value(QStringLiteral("dictionarycontentzoom")).toInt(100);
    m_zoom = std::clamp(savedZoom, 80, 160);

    m_restoring = true;
    m_multiAction->setChecked(m_preferences.value(QStringLiteral("multimode")).toBool());
    m_dictionaryList->setSelectionMode(m_multiAction->isChecked()
        ? QAbstractItemView::ExtendedSelection
        : QAbstractItemView::SingleSelection);
    populateDictionaries(m_dictionaries,
                         m_preferences.value(QStringLiteral("selecteddictid")).toString());
    m_restoring = false;

    m_ready = true;
    setBackendEnabled(true);
    setWindowTitle(tr("Medict %1").arg(m_backendVersion));
    updateDictionaryStatus();

    const QString lastWord = m_preferences.value(QStringLiteral("lastword")).toString();
    if (!lastWord.isEmpty() && !m_dictionaries.isEmpty()) {
        m_searchInput->setText(lastWord);
        QTimer::singleShot(120, this, [this, lastWord]() {
            performLookup(lastWord, true);
        });
    } else if (m_dictionaries.isEmpty()) {
        showWelcomePage();
        setStatus(tr("No dictionaries found. Open Settings to check the dictionary folder."));
    } else {
        showWelcomePage();
        setStatus(tr("Ready"));
    }
}

void MainWindow::populateDictionaries(const QJsonArray &dictionaries, const QString &preferredId)
{
    const QSignalBlocker blocker(m_dictionaryList);
    m_dictionaryList->clear();
    m_dictionaries = dictionaries;

    QStringList wanted;
    const QJsonArray savedIds = m_preferences.value(QStringLiteral("multidictids")).toArray();
    for (const QJsonValue &value : savedIds) {
        wanted.append(value.toString());
    }

    QListWidgetItem *preferredItem = nullptr;
    for (const QJsonValue &value : dictionaries) {
        const QJsonObject dictionary = value.toObject();
        const QString id = dictionary.value(QStringLiteral("id")).toString();
        const QString name = dictionary.value(QStringLiteral("name")).toString();
        const QString type = dictionary.value(QStringLiteral("dict_type")).toString();
        const QString directory = dictionary.value(QStringLiteral("base_dir")).toString();

        auto *item = new QListWidgetItem(name.isEmpty() ? id : name, m_dictionaryList);
        item->setData(Qt::UserRole, id);
        item->setToolTip(tr("%1\nType: %2\nFolder: %3")
                             .arg(name.isEmpty() ? id : name, type, directory));
        if (id == preferredId) {
            preferredItem = item;
        }
    }

    if (m_multiAction->isChecked()) {
        for (int row = 0; row < m_dictionaryList->count(); ++row) {
            QListWidgetItem *item = m_dictionaryList->item(row);
            if (wanted.contains(item->data(Qt::UserRole).toString())) {
                item->setSelected(true);
            }
        }
        if (!preferredItem) {
            preferredItem = m_dictionaryList->selectedItems().isEmpty()
                ? (m_dictionaryList->count() > 0 ? m_dictionaryList->item(0) : nullptr)
                : m_dictionaryList->selectedItems().first();
        }
    }

    if (!preferredItem && m_dictionaryList->count() > 0) {
        preferredItem = m_dictionaryList->item(0);
    }
    if (preferredItem) {
        m_dictionaryList->setCurrentItem(preferredItem);
        if (m_multiAction->isChecked()) {
            m_dictionaryList->clearSelection();
            for (int row = 0; row < m_dictionaryList->count(); ++row) {
                QListWidgetItem *item = m_dictionaryList->item(row);
                if (wanted.contains(item->data(Qt::UserRole).toString())) {
                    item->setSelected(true);
                }
            }
            preferredItem->setSelected(true);
        }
        m_selectedDictId = preferredItem->data(Qt::UserRole).toString();
    } else {
        m_selectedDictId.clear();
    }
    updateDictionaryStatus();
}

void MainWindow::updateDictionaryStatus()
{
    const int selected = selectedDictIds().size();
    const int total = m_dictionaryList->count();
    if (total == 0) {
        m_dictionaryStatusLabel->setText(tr("No dictionaries"));
    } else {
        m_dictionaryStatusLabel->setText(tr("%1 of %2 dictionaries").arg(selected).arg(total));
    }
}

QString MainWindow::currentDictId() const
{
    if (m_dictionaryList->currentItem()) {
        return m_dictionaryList->currentItem()->data(Qt::UserRole).toString();
    }
    const QList<QListWidgetItem *> selected = m_dictionaryList->selectedItems();
    return selected.isEmpty() ? QString() : selected.first()->data(Qt::UserRole).toString();
}

QStringList MainWindow::selectedDictIds() const
{
    QStringList ids;
    const QList<QListWidgetItem *> selected = m_dictionaryList->selectedItems();
    for (QListWidgetItem *item : selected) {
        const QString id = item->data(Qt::UserRole).toString();
        if (!id.isEmpty() && !ids.contains(id)) {
            ids.append(id);
        }
    }
    if (ids.isEmpty()) {
        const QString current = currentDictId();
        if (!current.isEmpty()) {
            ids.append(current);
        }
    }
    return ids;
}

QString MainWindow::dictionaryName(const QString &dictId) const
{
    for (const QJsonValue &value : m_dictionaries) {
        const QJsonObject dictionary = value.toObject();
        if (dictionary.value(QStringLiteral("id")).toString() == dictId) {
            const QString name = dictionary.value(QStringLiteral("name")).toString();
            return name.isEmpty() ? dictId : name;
        }
    }
    return dictId;
}

void MainWindow::selectDictionary(const QString &dictId)
{
    const QSignalBlocker blocker(m_dictionaryList);
    for (int row = 0; row < m_dictionaryList->count(); ++row) {
        QListWidgetItem *item = m_dictionaryList->item(row);
        if (item->data(Qt::UserRole).toString() != dictId) {
            continue;
        }
        if (!m_multiAction->isChecked()) {
            m_dictionaryList->clearSelection();
        }
        m_dictionaryList->setCurrentItem(item);
        item->setSelected(true);
        m_selectedDictId = dictId;
        updateDictionaryStatus();
        return;
    }
}

void MainWindow::scheduleSuggestions()
{
    if (!m_ready) {
        return;
    }
    m_suggestionTimer->start();
}

void MainWindow::requestSuggestions()
{
    const QString dictId = currentDictId();
    const QString word = m_searchInput->text().trimmed();
    if (dictId.isEmpty() || word.isEmpty()) {
        m_completionModel->setStringList({});
        m_matches->clear();
        return;
    }

    const int requestId = ++m_suggestionRequest;
    const QString path = encodedQuery(QStringLiteral("/search"), {
        {QStringLiteral("dict_id"), dictId},
        {QStringLiteral("word"), word},
    });
    m_api->get(path, [this, requestId](const QJsonValue &data, const QString &error) {
        if (requestId != m_suggestionRequest) {
            return;
        }
        if (!error.isEmpty() || !data.isArray()) {
            m_completionModel->setStringList({});
            m_matches->clear();
            return;
        }
        const QJsonArray entries = data.toArray();
        updateMatches(entries);

        QStringList words;
        for (const QJsonValue &value : entries) {
            const QString keyword = value.toObject().value(QStringLiteral("keyword")).toString();
            if (!keyword.isEmpty() && !words.contains(keyword)) {
                words.append(keyword);
            }
            if (words.size() >= 50) {
                break;
            }
        }
        m_completionModel->setStringList(words);
        if (!words.isEmpty() && m_searchInput->hasFocus()) {
            m_completer->complete();
        }
    });
}

void MainWindow::updateMatches(const QJsonArray &entries)
{
    m_matches->clear();
    const QString dictId = currentDictId();
    QStringList seen;
    for (const QJsonValue &value : entries) {
        const QJsonObject entry = value.toObject();
        const QString keyword = entry.value(QStringLiteral("keyword")).toString();
        if (keyword.isEmpty() || seen.contains(keyword)) {
            continue;
        }
        seen.append(keyword);
        auto *item = new QListWidgetItem(keyword, m_matches);
        item->setData(Qt::UserRole, dictId);
        item->setData(Qt::UserRole + 1,
                      QJsonDocument(entry).toJson(QJsonDocument::Compact));
    }
}

void MainWindow::performLookup(const QString &word, bool pushHistory)
{
    const QString trimmed = word.trimmed();
    if (!m_ready || trimmed.isEmpty()) {
        return;
    }
    const QStringList dictIds = selectedDictIds();
    if (dictIds.isEmpty()) {
        setStatus(tr("Select a dictionary first."));
        return;
    }

    if (pushHistory) {
        appendHistory(trimmed);
    }
    m_currentWord = trimmed;
    if (m_searchInput->text() != trimmed) {
        m_searchInput->setText(trimmed);
    }

    QJsonObject patch;
    patch.insert(QStringLiteral("lastword"), trimmed);
    savePreferences(patch);

    const int requestId = ++m_lookupRequest;
    auto batch = QSharedPointer<LookupBatch>::create();
    batch->requestId = requestId;
    batch->word = trimmed;
    batch->remaining = dictIds.size();
    for (const QString &dictId : dictIds) {
        batch->items.append(LookupItem{dictId, dictionaryName(dictId), {}, {}});
    }

    setStatus(tr("Searching for %1...").arg(trimmed));
    m_matches->clear();

    for (int index = 0; index < dictIds.size(); ++index) {
        const QString dictId = dictIds.at(index);
        const QString path = encodedQuery(QStringLiteral("/search"), {
            {QStringLiteral("dict_id"), dictId},
            {QStringLiteral("word"), trimmed},
        });
        m_api->get(path, [this, batch, index](const QJsonValue &data, const QString &error) {
            if (batch->requestId != m_lookupRequest) {
                return;
            }
            LookupItem &item = batch->items[index];
            item.error = error;
            if (error.isEmpty() && data.isArray()) {
                item.entries = data.toArray();
            }
            --batch->remaining;
            if (batch->remaining != 0) {
                return;
            }

            QJsonArray results;
            for (const LookupItem &result : batch->items) {
                QJsonObject resultObject;
                resultObject.insert(QStringLiteral("dictId"), result.dictId);
                resultObject.insert(QStringLiteral("dictName"), result.dictName);
                resultObject.insert(QStringLiteral("entries"), result.entries);
                resultObject.insert(QStringLiteral("error"), result.error);
                results.append(resultObject);
            }
            renderLookupBatch(batch->word, results);
        });
    }
}

void MainWindow::renderLookupBatch(const QString &word, const QJsonArray &results)
{
    while (m_definitionTabs->count() > 0) {
        QWidget *page = m_definitionTabs->widget(0);
        m_definitionTabs->removeTab(0);
        delete page;
    }

    QJsonArray primaryEntries;
    for (const QJsonValue &value : results) {
        const QJsonObject row = value.toObject();
        const QString dictId = row.value(QStringLiteral("dictId")).toString();
        const QString dictName = row.value(QStringLiteral("dictName")).toString();
        const QJsonArray entries = row.value(QStringLiteral("entries")).toArray();
        const QString error = row.value(QStringLiteral("error")).toString();

        if (primaryEntries.isEmpty() && !entries.isEmpty()) {
            primaryEntries = entries;
        }

        if (!error.isEmpty()) {
            auto *page = new QWidget(m_definitionTabs);
            auto *layout = new QVBoxLayout(page);
            auto *label = new QLabel(tr("Lookup failed in %1:\n%2").arg(dictName, error), page);
            label->setWordWrap(true);
            label->setAlignment(Qt::AlignTop | Qt::AlignLeft);
            layout->addWidget(label);
            m_definitionTabs->addTab(page, dictName);
            continue;
        }

        if (entries.isEmpty()) {
            auto *page = new QWidget(m_definitionTabs);
            auto *layout = new QVBoxLayout(page);
            auto *label = new QLabel(tr("No match for \"%1\" in %2.").arg(word, dictName), page);
            label->setAlignment(Qt::AlignCenter);
            layout->addWidget(label);
            m_definitionTabs->addTab(page, dictName);
            continue;
        }

        auto *view = new QWebEngineView(m_definitionTabs);
        auto *page = new DefinitionPage(view);
        view->setPage(page);
        view->settings()->setAttribute(QWebEngineSettings::JavascriptEnabled, true);
        view->settings()->setAttribute(QWebEngineSettings::LocalStorageEnabled, true);
        connect(page, &DefinitionPage::wordRequested, this, [this](const QString &nextWord) {
            if (!nextWord.isEmpty()) {
                m_searchInput->setText(nextWord);
                performLookup(nextWord, true);
            }
        });
        connect(page, &QWebEnginePage::loadFinished, view, [this, view](bool) {
            view->setZoomFactor(m_zoom / 100.0);
        });
        view->setZoomFactor(m_zoom / 100.0);
        view->setUrl(buildEntryUrl(dictId, entries.first().toObject()));
        m_definitionTabs->addTab(view, dictName);
    }

    if (m_definitionTabs->count() == 0) {
        showWelcomePage();
    } else {
        m_definitionTabs->setCurrentIndex(0);
    }
    updateMatches(primaryEntries);
    if (m_definitionTabs->count() == 1) {
        setStatus(tr("%1 — %2").arg(word, m_definitionTabs->tabText(0)));
    } else {
        setStatus(tr("%1 — %2 dictionaries").arg(word).arg(m_definitionTabs->count()));
    }
}

QUrl MainWindow::buildEntryUrl(const QString &dictId, const QJsonObject &entry) const
{
    QUrl url(m_staticBaseUrl + QStringLiteral("/__tcidem_query"));
    QUrlQuery query;
    query.addQueryItem(QStringLiteral("dict_id"), dictId);
    query.addQueryItem(QStringLiteral("keyword"),
                       entry.value(QStringLiteral("keyword")).toString());
    query.addQueryItem(QStringLiteral("entry_id"), QStringLiteral("0"));

    const QStringList numericFields = {
        QStringLiteral("record_start_offset"),
        QStringLiteral("record_end_offset"),
        QStringLiteral("record_block_data_start_offset"),
        QStringLiteral("record_block_data_compress_size"),
        QStringLiteral("record_block_data_decompress_size"),
        QStringLiteral("keyword_data_start_offset"),
        QStringLiteral("keyword_data_end_offset"),
    };
    for (const QString &field : numericFields) {
        const qint64 value =
            static_cast<qint64>(entry.value(field).toDouble());
        query.addQueryItem(field, QString::number(value));
    }
    url.setQuery(query);
    return url;
}

void MainWindow::showWelcomePage()
{
    while (m_definitionTabs->count() > 0) {
        QWidget *page = m_definitionTabs->widget(0);
        m_definitionTabs->removeTab(0);
        delete page;
    }
    auto *page = new QWidget(m_definitionTabs);
    auto *layout = new QVBoxLayout(page);
    auto *title = new QLabel(QStringLiteral("<h1>Medict</h1>"), page);
    title->setAlignment(Qt::AlignCenter);
    auto *hint = new QLabel(
        tr("<p>Type a word and press Enter.</p>"
           "<p>Select more than one dictionary and turn on <b>Multi</b> "
           "to compare results.</p>"),
        page);
    hint->setWordWrap(true);
    hint->setAlignment(Qt::AlignCenter);
    layout->addStretch();
    layout->addWidget(title);
    layout->addWidget(hint);
    layout->addStretch();
    m_definitionTabs->addTab(page, tr("Welcome"));
}

void MainWindow::appendHistory(const QString &word)
{
    if (word.isEmpty()) {
        return;
    }
    if (m_historyPosition >= 0 && m_historyPosition < m_history.size()
        && m_history.at(m_historyPosition) == word) {
        return;
    }
    while (m_history.size() > m_historyPosition + 1) {
        m_history.removeLast();
    }
    m_history.append(word);
    if (m_history.size() > 100) {
        m_history.removeFirst();
    }
    m_historyPosition = m_history.size() - 1;
    updateHistoryActions();
}

void MainWindow::updateHistoryActions()
{
    m_backAction->setEnabled(m_historyPosition > 0);
    m_forwardAction->setEnabled(m_historyPosition >= 0
                                && m_historyPosition < m_history.size() - 1);
}

void MainWindow::navigateHistory(int offset)
{
    const int next = m_historyPosition + offset;
    if (next < 0 || next >= m_history.size()) {
        return;
    }
    m_historyPosition = next;
    const QString word = m_history.at(next);
    m_searchInput->setText(word);
    performLookup(word, false);
    updateHistoryActions();
}

void MainWindow::openSettings()
{
    SettingsDialog dialog(m_api, m_baseDictDir, this);
    connect(&dialog, &SettingsDialog::dictionariesChanged,
            this, &MainWindow::refreshDictionaries);
    dialog.exec();
}

void MainWindow::openBookmarks()
{
    BookmarksDialog dialog(m_api, this);
    QString selectedWord;
    QString selectedDictId;
    connect(&dialog, &BookmarksDialog::activateWord,
            this, [&selectedWord, &selectedDictId](const QString &word, const QString &dictId) {
        selectedWord = word;
        selectedDictId = dictId;
    });
    dialog.exec();

    if (!selectedWord.isEmpty()) {
        if (!selectedDictId.isEmpty()) {
            selectDictionary(selectedDictId);
        }
        m_searchInput->setText(selectedWord);
        performLookup(selectedWord, true);
    }
}

void MainWindow::openCssEditor()
{
    const QString dictId = currentDictId();
    if (dictId.isEmpty()) {
        setStatus(tr("Select a dictionary first."));
        return;
    }
    if (m_cssEditor) {
        m_cssEditor->raise();
        m_cssEditor->activateWindow();
        return;
    }

    auto *dialog = new CssEditorDialog(m_api, dictId, dictionaryName(dictId),
                                       m_currentWord, this);
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    connect(dialog, &CssEditorDialog::previewCss, this, &MainWindow::applyUserCss);
    connect(dialog, &QDialog::finished, this, [this]() {
        m_cssEditor = nullptr;
        reloadTabs();
    });
    m_cssEditor = dialog;
    dialog->show();
}

void MainWindow::addBookmark()
{
    const QString word = m_currentWord.trimmed();
    const QString dictId = currentDictId();
    if (word.isEmpty() || dictId.isEmpty()) {
        setStatus(tr("Look up a word before you save it."));
        return;
    }
    m_api->post(QStringLiteral("/bookmarks/add"), QJsonObject{
        {QStringLiteral("word"), word},
        {QStringLiteral("dictId"), dictId},
        {QStringLiteral("notebookId"), QString()},
    }, [this, word](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(error);
            return;
        }
        setStatus(tr("Saved \"%1\" to the default notebook.").arg(word));
    });
}

void MainWindow::exportCurrentEntry()
{
    const QString dictId = currentDictId();
    if (m_currentWord.isEmpty() || dictId.isEmpty()) {
        setStatus(tr("Look up a word before you export it."));
        return;
    }
    const QString suggested = QDir(QStandardPaths::writableLocation(
        QStandardPaths::DocumentsLocation)).filePath(m_currentWord + QStringLiteral(".html"));
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Export entry"), suggested, tr("HTML files (*.html)"));
    if (path.isEmpty()) {
        return;
    }

    const QString requestPath = encodedQuery(QStringLiteral("/snapshot"), {
        {QStringLiteral("dictId"), dictId},
        {QStringLiteral("word"), m_currentWord},
    });
    m_api->get(requestPath, [this, path](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isString()) {
            setStatus(error.isEmpty() ? tr("The entry was not found.") : error);
            return;
        }
        QSaveFile file(path);
        if (!file.open(QIODevice::WriteOnly)
            || file.write(data.toString().toUtf8()) < 0
            || !file.commit()) {
            showError(tr("Export error"), tr("Cannot write %1.").arg(path));
            return;
        }
        setStatus(tr("Exported %1").arg(path));
    });
}

void MainWindow::exportAnki()
{
    const QString suggested = QDir(QStandardPaths::writableLocation(
        QStandardPaths::DocumentsLocation)).filePath(QStringLiteral("medict.apkg"));
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Export Anki package"), suggested, tr("Anki packages (*.apkg)"));
    if (path.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/anki"), QJsonObject{
        {QStringLiteral("notebookId"), QString()},
        {QStringLiteral("path"), path},
    }, [this, path](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(error);
            return;
        }
        setStatus(tr("Exported %1").arg(path));
    });
}

void MainWindow::refreshDictionaries()
{
    if (!m_ready) {
        return;
    }
    const QString preferred = currentDictId();
    setStatus(tr("Scanning the dictionary folder..."));
    m_api->post(QStringLiteral("/dicts/refresh"), {}, [this, preferred](
        const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isArray()) {
            setStatus(error.isEmpty() ? tr("The dictionary list is invalid.") : error);
            return;
        }
        const bool wasRestoring = m_restoring;
        m_restoring = true;
        populateDictionaries(data.toArray(), preferred);
        m_restoring = wasRestoring;
        setStatus(tr("Dictionaries refreshed."));
        if (!m_currentWord.isEmpty()) {
            performLookup(m_currentWord, false);
        }
    });
}

void MainWindow::rebuildCurrentIndex()
{
    const QString dictId = currentDictId();
    if (dictId.isEmpty()) {
        setStatus(tr("Select a dictionary first."));
        return;
    }
    if (QMessageBox::question(this, tr("Rebuild index"),
                              tr("Rebuild the keyword index for \"%1\"?")
                                  .arg(dictionaryName(dictId)))
        != QMessageBox::Yes) {
        return;
    }
    setStatus(tr("Rebuilding the index..."));
    m_api->post(QStringLiteral("/dicts/index"), QJsonObject{
        {QStringLiteral("dictId"), dictId},
    }, [this](const QJsonValue &, const QString &error) {
        setStatus(error.isEmpty() ? tr("Index rebuilt.") : error);
    });
}

void MainWindow::openDictionaryFolder()
{
    if (m_baseDictDir.isEmpty()) {
        return;
    }
    QDesktopServices::openUrl(QUrl::fromLocalFile(m_baseDictDir));
}

void MainWindow::showAbout()
{
    QMessageBox::about(
        this,
        tr("About Medict"),
        tr("<h3>Medict %1</h3>"
           "<p>Offline dictionary for MDict and StarDict files.</p>"
           "<p>Qt %2 frontend. Go dictionary engine %3.</p>"
           "<p>License: GNU GPL v3 or later.</p>")
            .arg(m_version, QString::fromLatin1(qVersion()), m_backendVersion));
}

void MainWindow::setZoom(int zoom)
{
    m_zoom = std::clamp(zoom, 80, 160);
    const QList<QWebEngineView *> views = m_definitionTabs->findChildren<QWebEngineView *>();
    for (QWebEngineView *view : views) {
        view->setZoomFactor(m_zoom / 100.0);
    }
    savePreferences(QJsonObject{
        {QStringLiteral("dictionarycontentzoom"), m_zoom},
    });
    setStatus(tr("Definition zoom: %1%").arg(m_zoom));
}

void MainWindow::applyUserCss(const QString &css)
{
    const QString cssLiteral = jsonStringLiteral(css);
    const QString script = QStringLiteral(
        "(function(){"
        "var old=document.getElementById('medict-qt-user-css');"
        "if(old){old.remove();}"
        "if(%1.length){"
        "var style=document.createElement('style');"
        "style.id='medict-qt-user-css';"
        "style.textContent=%1;"
        "document.head.appendChild(style);"
        "}"
        "})();").arg(cssLiteral);

    const QList<QWebEngineView *> views = m_definitionTabs->findChildren<QWebEngineView *>();
    for (QWebEngineView *view : views) {
        view->page()->runJavaScript(script);
    }
}

void MainWindow::reloadTabs()
{
    const QList<QWebEngineView *> views = m_definitionTabs->findChildren<QWebEngineView *>();
    for (QWebEngineView *view : views) {
        view->reload();
    }
}

void MainWindow::savePreferences(const QJsonObject &patch)
{
    for (auto it = patch.constBegin(); it != patch.constEnd(); ++it) {
        m_preferences.insert(it.key(), it.value());
    }
    if (!m_ready) {
        return;
    }
    m_api->post(QStringLiteral("/preferences"), m_preferences,
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(tr("Cannot save preferences: %1").arg(error));
        }
    });
}

void MainWindow::setStatus(const QString &message)
{
    m_statusLabel->setText(message);
}

void MainWindow::showError(const QString &title, const QString &message)
{
    QMessageBox::critical(this, title, message);
}
