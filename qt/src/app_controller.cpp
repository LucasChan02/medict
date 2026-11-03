// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "app_controller.h"

#include "api_client.h"

#include <QDateTime>
#include <QDesktopServices>
#include <QDir>
#include <QLocale>
#include <QMap>
#include <QSaveFile>
#include <QSharedPointer>
#include <QStandardPaths>
#include <QTimer>
#include <QUrlQuery>
#include <QVector>

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

QString encodedQuery(const QString &path, const QMap<QString, QString> &values)
{
    QUrlQuery query;
    for (auto it = values.cbegin(); it != values.cend(); ++it) {
        query.addQueryItem(it.key(), it.value());
    }
    return path + QLatin1Char('?') + query.query(QUrl::FullyEncoded);
}

QString preferenceString(const QJsonObject &preferences, const QString &key,
                         const QString &fallback = QString())
{
    const QJsonValue value = preferences.value(key);
    if (value.isString()) {
        return value.toString();
    }
    if (value.isDouble()) {
        return QString::number(value.toInt());
    }
    return fallback;
}

} // namespace

AppController::AppController(ApiClient *api, const QString &version, QObject *parent)
    : QObject(parent)
    , m_api(api)
    , m_version(version)
    , m_backendVersion(version)
{
    setStatus(tr("Starting the dictionary engine..."));

    connect(m_api, &ApiClient::ready, this, &AppController::loadBootstrap);
    connect(m_api, &ApiClient::failed, this, [this](const QString &message) {
        setBackendUnavailable(message);
    });
    connect(m_api, &ApiClient::backendStopped, this, [this](const QString &message) {
        setReady(false);
        setStatus(message);
    });
}

void AppController::start()
{
    QTimer::singleShot(0, m_api, [this]() {
        m_api->start(m_version);
    });
}

void AppController::dismissError()
{
    if (!m_errorVisible) {
        return;
    }
    m_errorVisible = false;
    emit errorChanged();
}

void AppController::setSearchText(const QString &text)
{
    if (m_searchText == text) {
        return;
    }
    m_searchText = text;
    emit searchTextChanged();
}

void AppController::requestSuggestions(const QString &prefix)
{
    if (!m_ready) {
        return;
    }

    const QString dictId = currentDictionaryId();
    const QString word = prefix.trimmed();
    if (dictId.isEmpty() || word.isEmpty()) {
        if (!m_suggestions.isEmpty()) {
            m_suggestions.clear();
            emit suggestionsChanged();
        }
        if (!m_matches.isEmpty()) {
            m_matches.clear();
            emit matchesChanged();
        }
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
            m_suggestions.clear();
            m_matches.clear();
            emit suggestionsChanged();
            emit matchesChanged();
            return;
        }

        const QJsonArray entries = data.toArray();
        updateMatches(entries);

        QVariantList words;
        QStringList seen;
        for (const QJsonValue &value : entries) {
            const QString keyword = value.toObject().value(QStringLiteral("keyword")).toString();
            if (keyword.isEmpty() || seen.contains(keyword)) {
                continue;
            }
            seen.append(keyword);
            words.append(keyword);
            if (words.size() >= 50) {
                break;
            }
        }
        m_suggestions = words;
        emit suggestionsChanged();
    }, this);
}

void AppController::useSuggestion(const QString &word)
{
    if (word.trimmed().isEmpty()) {
        return;
    }
    setSearchText(word);
    lookup(word);
}

void AppController::lookup(const QString &word)
{
    const QString trimmed = word.trimmed();
    if (!m_ready || trimmed.isEmpty()) {
        return;
    }

    normalizeSelection();
    if (m_selectedDictIds.isEmpty()) {
        setStatus(tr("Select a dictionary first."));
        return;
    }

    appendHistory(trimmed);
    m_currentWord = trimmed;
    emit currentWordChanged();
    setSearchText(trimmed);

    QJsonObject patch;
    patch.insert(QStringLiteral("lastword"), trimmed);
    savePreferences(patch);

    const int requestId = ++m_lookupRequest;
    auto batch = QSharedPointer<LookupBatch>::create();
    batch->requestId = requestId;
    batch->word = trimmed;
    batch->remaining = m_selectedDictIds.size();
    for (const QString &dictId : m_selectedDictIds) {
        batch->items.append(LookupItem{dictId, dictionaryName(dictId), {}, {}});
    }

    m_matches.clear();
    emit matchesChanged();
    setStatus(tr("Searching for %1...").arg(trimmed));

    for (int index = 0; index < m_selectedDictIds.size(); ++index) {
        const QString dictId = m_selectedDictIds.at(index);
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
        }, this);
    }
}

void AppController::renderLookupBatch(const QString &word, const QJsonArray &results)
{
    QVariantList definitions;
    QJsonArray primaryEntries;
    QStringList renderedDictionaries;

    for (const QJsonValue &value : results) {
        const QJsonObject row = value.toObject();
        const QString dictId = row.value(QStringLiteral("dictId")).toString();
        const QString dictName = row.value(QStringLiteral("dictName")).toString();
        const QJsonArray entries = row.value(QStringLiteral("entries")).toArray();
        const QString error = row.value(QStringLiteral("error")).toString();

        if (primaryEntries.isEmpty() && !entries.isEmpty()) {
            primaryEntries = entries;
        }
        renderedDictionaries.append(dictName);

        QVariantMap definition;
        definition.insert(QStringLiteral("dictId"), dictId);
        definition.insert(QStringLiteral("dictName"), dictName);
        definition.insert(QStringLiteral("hasEntry"), false);
        definition.insert(QStringLiteral("url"), QString());

        if (!error.isEmpty()) {
            definition.insert(QStringLiteral("message"),
                              tr("Lookup failed in %1.\n\n%2").arg(dictName, error));
        } else if (entries.isEmpty()) {
            definition.insert(QStringLiteral("message"),
                              tr("No match for \"%1\" in %2.").arg(word, dictName));
        } else {
            definition.insert(QStringLiteral("hasEntry"), true);
            definition.insert(QStringLiteral("url"),
                              buildEntryUrl(dictId, entries.first().toObject()).toString());
        }
        definitions.append(definition);
    }

    m_definitions = definitions;
    m_currentDefinitionIndex = definitions.isEmpty() ? -1 : 0;
    emit definitionsChanged();
    emit currentDefinitionIndexChanged();
    updateMatches(primaryEntries);

    if (renderedDictionaries.isEmpty()) {
        setStatus(tr("No dictionary returned a result."));
    } else if (renderedDictionaries.size() == 1) {
        setStatus(tr("%1 — %2").arg(word, renderedDictionaries.first()));
    } else {
        setStatus(tr("%1 — %2 dictionaries").arg(word).arg(renderedDictionaries.size()));
    }
}

QUrl AppController::buildEntryUrl(const QString &dictId, const QJsonObject &entry) const
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
        const qint64 value = static_cast<qint64>(entry.value(field).toDouble());
        query.addQueryItem(field, QString::number(value));
    }
    url.setQuery(query);
    return url;
}

void AppController::updateMatches(const QJsonArray &entries)
{
    const QString dictId = currentDictionaryId();
    QVariantList matches;
    QStringList seen;
    for (const QJsonValue &value : entries) {
        const QJsonObject entry = value.toObject();
        const QString keyword = entry.value(QStringLiteral("keyword")).toString();
        if (keyword.isEmpty() || seen.contains(keyword)) {
            continue;
        }
        seen.append(keyword);
        matches.append(QVariantMap{
            {QStringLiteral("keyword"), keyword},
            {QStringLiteral("dictId"), dictId},
        });
    }
    m_matches = matches;
    emit matchesChanged();
}

void AppController::clearLookupState()
{
    m_suggestions.clear();
    m_matches.clear();
    m_definitions.clear();
    m_currentDefinitionIndex = -1;
    emit suggestionsChanged();
    emit matchesChanged();
    emit definitionsChanged();
    emit currentDefinitionIndexChanged();
}

void AppController::loadBootstrap()
{
    setStatus(tr("Loading dictionaries..."));
    m_api->get(QStringLiteral("/bootstrap"), [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isObject()) {
            setBackendUnavailable(error.isEmpty()
                                      ? tr("The backend bootstrap response is invalid.")
                                      : error);
            return;
        }
        applyBootstrap(data.toObject());
    }, this);
}

void AppController::applyBootstrap(const QJsonObject &bootstrap)
{
    m_backendVersion = bootstrap.value(QStringLiteral("version")).toString(m_version);
    m_staticBaseUrl = bootstrap.value(QStringLiteral("staticBaseUrl")).toString();
    if (m_staticBaseUrl.isEmpty()) {
        m_staticBaseUrl = m_api->staticBaseUrl();
    }
    m_baseDictDir = bootstrap.value(QStringLiteral("baseDictDir")).toString();
    m_preferences = bootstrap.value(QStringLiteral("preferences")).toObject();
    m_dictionaries = bootstrap.value(QStringLiteral("dictionaries")).toArray();

    const int savedZoom =
        m_preferences.value(QStringLiteral("dictionarycontentzoom")).toInt(100);
    m_zoom = std::clamp(savedZoom, 80, 160);
    m_multiMode = m_preferences.value(QStringLiteral("multimode")).toBool();
    restoreDictionarySelection(
        m_preferences.value(QStringLiteral("selecteddictid")).toString());

    emit backendVersionChanged();
    emit baseDictDirChanged();
    emit zoomChanged();
    emit multiModeChanged();
    rebuildDictionaryRows();
    emit dictionariesChanged();
    emit selectionChanged();
    setReady(true);

    const QString lastWord = m_preferences.value(QStringLiteral("lastword")).toString();
    if (!lastWord.isEmpty() && !m_dictionaries.isEmpty()) {
        setSearchText(lastWord);
        QTimer::singleShot(120, this, [this, lastWord]() {
            lookup(lastWord);
        });
    } else if (m_dictionaries.isEmpty()) {
        setStatus(tr("No dictionaries found. Open Settings to check the dictionary folder."));
    } else {
        setStatus(tr("Ready"));
    }
}

void AppController::setReady(bool ready)
{
    if (m_ready == ready) {
        return;
    }
    m_ready = ready;
    emit readyChanged();
}

void AppController::setBackendUnavailable(const QString &message)
{
    setReady(false);
    setStatus(tr("The dictionary engine is not available."));
    showError(tr("Dictionary engine error"), message);
}

void AppController::setStatus(const QString &message)
{
    if (m_statusText == message) {
        return;
    }
    m_statusText = message;
    emit statusTextChanged();
}

void AppController::showError(const QString &title, const QString &message)
{
    m_errorTitle = title;
    m_errorText = message;
    m_errorVisible = true;
    emit errorChanged();
}

void AppController::rebuildDictionaryRows()
{
    QVariantList rows;
    rows.reserve(m_dictionaries.size());
    for (const QJsonValue &value : m_dictionaries) {
        const QJsonObject dictionary = value.toObject();
        const QString id = dictionary.value(QStringLiteral("id")).toString();
        const QString name = dictionary.value(QStringLiteral("name")).toString();
        const QString type = dictionary.value(QStringLiteral("dict_type")).toString();
        const QString directory = dictionary.value(QStringLiteral("base_dir")).toString();
        QVariantMap row;
        row.insert(QStringLiteral("id"), id);
        row.insert(QStringLiteral("name"), name.isEmpty() ? id : name);
        row.insert(QStringLiteral("type"), type);
        row.insert(QStringLiteral("directory"), directory);
        row.insert(QStringLiteral("selected"), m_selectedDictIds.contains(id));
        row.insert(QStringLiteral("current"), id == m_currentDictId);
        row.insert(QStringLiteral("tooltip"),
                   tr("%1\nType: %2\nFolder: %3")
                       .arg(name.isEmpty() ? id : name, type, directory));
        rows.append(row);
    }
    m_dictionaryRows = rows;
}

void AppController::restoreDictionarySelection(const QString &preferredId)
{
    QStringList availableIds;
    for (const QJsonValue &value : m_dictionaries) {
        availableIds.append(value.toObject().value(QStringLiteral("id")).toString());
    }

    QStringList wanted;
    const QJsonArray savedIds = m_preferences.value(QStringLiteral("multidictids")).toArray();
    for (const QJsonValue &value : savedIds) {
        const QString id = value.toString();
        if (availableIds.contains(id) && !wanted.contains(id)) {
            wanted.append(id);
        }
    }

    if (m_multiMode && !wanted.isEmpty()) {
        m_selectedDictIds = wanted;
        if (availableIds.contains(preferredId)) {
            m_currentDictId = preferredId;
        } else {
            m_currentDictId = m_selectedDictIds.first();
        }
    } else if (availableIds.contains(preferredId)) {
        m_currentDictId = preferredId;
        m_selectedDictIds = {preferredId};
    } else if (!availableIds.isEmpty()) {
        m_currentDictId = availableIds.first();
        m_selectedDictIds = {m_currentDictId};
    } else {
        m_currentDictId.clear();
        m_selectedDictIds.clear();
    }
    normalizeSelection();
}

void AppController::normalizeSelection()
{
    QStringList availableIds;
    for (const QJsonValue &value : m_dictionaries) {
        availableIds.append(value.toObject().value(QStringLiteral("id")).toString());
    }

    QStringList filtered;
    for (const QString &id : m_selectedDictIds) {
        if (availableIds.contains(id) && !filtered.contains(id)) {
            filtered.append(id);
        }
    }
    if (filtered.isEmpty() && !availableIds.isEmpty()) {
        if (availableIds.contains(m_currentDictId)) {
            filtered.append(m_currentDictId);
        } else {
            filtered.append(availableIds.first());
        }
    }

    if (!m_multiMode && filtered.size() > 1) {
        filtered = {filtered.first()};
    }
    m_selectedDictIds = filtered;
    if (m_selectedDictIds.isEmpty()) {
        m_currentDictId.clear();
    } else if (!m_selectedDictIds.contains(m_currentDictId)) {
        m_currentDictId = m_selectedDictIds.first();
    }
}

QString AppController::dictionaryName(const QString &dictId) const
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

QString AppController::dictionaryStatus() const
{
    if (m_dictionaries.isEmpty()) {
        return tr("No dictionaries");
    }
    return tr("%1 of %2 dictionaries")
        .arg(m_selectedDictIds.size())
        .arg(m_dictionaries.size());
}

void AppController::selectDictionary(const QString &dictId)
{
    if (dictId.isEmpty()) {
        return;
    }

    bool exists = false;
    for (const QJsonValue &value : m_dictionaries) {
        if (value.toObject().value(QStringLiteral("id")).toString() == dictId) {
            exists = true;
            break;
        }
    }
    if (!exists) {
        return;
    }

    if (m_multiMode) {
        if (m_selectedDictIds.contains(dictId)) {
            if (m_selectedDictIds.size() > 1) {
                m_selectedDictIds.removeAll(dictId);
                m_currentDictId = m_selectedDictIds.last();
            } else {
                m_currentDictId = dictId;
            }
        } else {
            m_selectedDictIds.append(dictId);
            m_currentDictId = dictId;
        }
    } else {
        m_selectedDictIds = {dictId};
        m_currentDictId = dictId;
    }

    rebuildDictionaryRows();
    emit dictionariesChanged();
    emit selectionChanged();

    QJsonObject patch;
    patch.insert(QStringLiteral("selecteddictid"), m_currentDictId);
    QJsonArray activeIds;
    for (const QString &id : m_selectedDictIds) {
        activeIds.append(id);
    }
    patch.insert(QStringLiteral("multidictids"), activeIds);
    savePreferences(patch);

    if (!m_currentWord.isEmpty()) {
        lookup(m_currentWord);
    }
}

void AppController::setMultiMode(bool enabled)
{
    if (m_multiMode == enabled) {
        return;
    }
    m_multiMode = enabled;
    if (!enabled) {
        normalizeSelection();
    }
    rebuildDictionaryRows();
    emit multiModeChanged();
    emit dictionariesChanged();
    emit selectionChanged();

    QJsonObject patch;
    patch.insert(QStringLiteral("multimode"), enabled);
    patch.insert(QStringLiteral("selecteddictid"), m_currentDictId);
    QJsonArray activeIds;
    for (const QString &id : m_selectedDictIds) {
        activeIds.append(id);
    }
    patch.insert(QStringLiteral("multidictids"), activeIds);
    savePreferences(patch);

    if (!m_currentWord.isEmpty()) {
        lookup(m_currentWord);
    }
}

void AppController::refreshDictionaries()
{
    if (!m_ready) {
        return;
    }
    const QString preferred = m_currentDictId;
    setStatus(tr("Scanning the dictionary folder..."));
    m_api->post(QStringLiteral("/dicts/refresh"), {},
                [this, preferred](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isArray()) {
            setStatus(error.isEmpty() ? tr("The dictionary list is invalid.") : error);
            return;
        }
        m_dictionaries = data.toArray();
        restoreDictionarySelection(preferred);
        rebuildDictionaryRows();
        emit dictionariesChanged();
        emit selectionChanged();
        setStatus(tr("Dictionaries refreshed."));
        if (!m_currentWord.isEmpty()) {
            lookup(m_currentWord);
        }
    }, this);
}

void AppController::rebuildCurrentIndex()
{
    const QString dictId = currentDictionaryId();
    if (dictId.isEmpty()) {
        setStatus(tr("Select a dictionary first."));
        return;
    }
    setStatus(tr("Rebuilding the index..."));
    m_api->post(QStringLiteral("/dicts/index"), QJsonObject{
        {QStringLiteral("dictId"), dictId},
    }, [this](const QJsonValue &, const QString &error) {
        setStatus(error.isEmpty() ? tr("Index rebuilt.") : error);
    }, this);
}

void AppController::openDictionaryFolder()
{
    if (m_baseDictDir.isEmpty()) {
        return;
    }
    QDesktopServices::openUrl(QUrl::fromLocalFile(m_baseDictDir));
}

void AppController::reloadViews()
{
    emit reloadRequested();
}

void AppController::appendHistory(const QString &word)
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
    emit historyChanged();
}

void AppController::navigateHistory(int offset)
{
    const int next = m_historyPosition + offset;
    if (next < 0 || next >= m_history.size()) {
        return;
    }
    m_historyPosition = next;
    const QString word = m_history.at(next);
    setSearchText(word);
    lookup(word);
    emit historyChanged();
}

void AppController::back()
{
    navigateHistory(-1);
}

void AppController::forward()
{
    navigateHistory(1);
}

bool AppController::canGoBack() const
{
    return m_historyPosition > 0;
}

bool AppController::canGoForward() const
{
    return m_historyPosition >= 0 && m_historyPosition < m_history.size() - 1;
}

void AppController::setZoom(int zoom)
{
    const int clamped = std::clamp(zoom, 80, 160);
    if (m_zoom == clamped) {
        return;
    }
    m_zoom = clamped;
    emit zoomChanged();
    savePreferences(QJsonObject{
        {QStringLiteral("dictionarycontentzoom"), m_zoom},
    });
    setStatus(tr("Definition zoom: %1%").arg(m_zoom));
}

void AppController::zoomIn()
{
    setZoom(m_zoom + 10);
}

void AppController::zoomOut()
{
    setZoom(m_zoom - 10);
}

void AppController::resetZoom()
{
    setZoom(100);
}

void AppController::addBookmark()
{
    const QString word = m_currentWord.trimmed();
    const QString dictId = currentDictionaryId();
    if (word.isEmpty() || dictId.isEmpty()) {
        setStatus(tr("Look up a word before you save it."));
        return;
    }
    m_api->post(QStringLiteral("/bookmarks/add"), QJsonObject{
        {QStringLiteral("word"), word},
        {QStringLiteral("dictId"), dictId},
        {QStringLiteral("notebookId"), QString()},
    }, [this, word](const QJsonValue &, const QString &error) {
        setStatus(error.isEmpty()
                      ? tr("Saved \"%1\" to the default notebook.").arg(word)
                      : error);
    }, this);
}

void AppController::setNotebooksBusy(bool busy)
{
    if (m_notebooksBusy == busy) {
        return;
    }
    m_notebooksBusy = busy;
    emit notebooksBusyChanged();
}

void AppController::loadNotebooks()
{
    if (!m_ready || m_notebooksBusy) {
        return;
    }
    setNotebooksBusy(true);
    m_api->get(QStringLiteral("/notebooks"), [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isArray()) {
            setNotebooksBusy(false);
            setStatus(error.isEmpty() ? tr("Cannot load notebooks.") : error);
            return;
        }
        populateNotebooks(data.toArray());

        m_api->get(QStringLiteral("/bookmarks"), [this](const QJsonValue &bookmarkData,
                                                        const QString &bookmarkError) {
            if (!bookmarkError.isEmpty() || !bookmarkData.isArray()) {
                setNotebooksBusy(false);
                setStatus(bookmarkError.isEmpty() ? tr("Cannot load saved words.")
                                                  : bookmarkError);
                return;
            }
            populateBookmarks(bookmarkData.toArray());
            setNotebooksBusy(false);
        }, this);
    }, this);
}

void AppController::populateNotebooks(const QJsonArray &notebooks)
{
    const QString previous = m_selectedNotebookId;
    QVariantList rows;
    QString defaultId;
    for (const QJsonValue &value : notebooks) {
        const QJsonObject notebook = value.toObject();
        const QString id = notebook.value(QStringLiteral("id")).toString();
        const QString name = notebook.value(QStringLiteral("name")).toString();
        const bool isDefault = notebook.value(QStringLiteral("is_default")).toBool();
        if (isDefault) {
            defaultId = id;
        }
        rows.append(QVariantMap{
            {QStringLiteral("id"), id},
            {QStringLiteral("name"), name},
            {QStringLiteral("isDefault"), isDefault},
            {QStringLiteral("displayName"), isDefault ? tr("%1 (default)").arg(name) : name},
        });
    }
    m_notebooks = rows;

    QString selected = previous;
    bool previousStillExists = false;
    for (const QVariant &row : m_notebooks) {
        if (row.toMap().value(QStringLiteral("id")).toString() == previous) {
            previousStillExists = true;
            break;
        }
    }
    if (!previousStillExists) {
        selected = defaultId;
    }
    m_selectedNotebookId = selected;
    emit notebooksChanged();
    emit selectedNotebookChanged();
}

void AppController::populateBookmarks(const QJsonArray &bookmarks)
{
    QVariantList rows;
    for (const QJsonValue &value : bookmarks) {
        const QJsonObject bookmark = value.toObject();
        const qint64 savedAt =
            static_cast<qint64>(bookmark.value(QStringLiteral("saved_at")).toDouble());
        rows.append(QVariantMap{
            {QStringLiteral("word"), bookmark.value(QStringLiteral("word")).toString()},
            {QStringLiteral("dictId"), bookmark.value(QStringLiteral("dict_id")).toString()},
            {QStringLiteral("dictName"), bookmark.value(QStringLiteral("dict_name")).toString()},
            {QStringLiteral("notebookId"),
             bookmark.value(QStringLiteral("notebook_id")).toString()},
            {QStringLiteral("savedAt"), savedAt},
            {QStringLiteral("savedAtText"),
             QDateTime::fromSecsSinceEpoch(savedAt).toString(QStringLiteral("yyyy-MM-dd HH:mm"))},
        });
    }
    m_bookmarks = rows;
    emit bookmarksChanged();
}

void AppController::selectNotebook(const QString &notebookId)
{
    if (m_selectedNotebookId == notebookId) {
        return;
    }
    m_selectedNotebookId = notebookId;
    emit selectedNotebookChanged();
}

void AppController::createNotebook(const QString &name)
{
    const QString trimmed = name.trimmed();
    if (trimmed.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/create"),
                QJsonObject{{QStringLiteral("name"), trimmed}},
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void AppController::renameNotebook(const QString &notebookId, const QString &name)
{
    const QString trimmed = name.trimmed();
    if (notebookId.isEmpty() || trimmed.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/rename"), QJsonObject{
        {QStringLiteral("id"), notebookId},
        {QStringLiteral("name"), trimmed},
    }, [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void AppController::deleteNotebook(const QString &notebookId)
{
    if (notebookId.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/delete"),
                QJsonObject{{QStringLiteral("id"), notebookId}},
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void AppController::setDefaultNotebook(const QString &notebookId)
{
    if (notebookId.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/default"),
                QJsonObject{{QStringLiteral("id"), notebookId}},
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void AppController::removeBookmark(const QString &word, const QString &dictId,
                                   const QString &notebookId)
{
    if (word.isEmpty() || dictId.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/bookmarks/remove"), QJsonObject{
        {QStringLiteral("word"), word},
        {QStringLiteral("dictId"), dictId},
        {QStringLiteral("notebookId"), notebookId},
    }, [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            setStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void AppController::activateBookmark(const QString &word, const QString &dictId)
{
    if (word.isEmpty()) {
        return;
    }
    if (!dictId.isEmpty()) {
        selectDictionary(dictId);
    }
    setSearchText(word);
    lookup(word);
}

void AppController::exportAnki(const QString &path, const QString &notebookId)
{
    if (path.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/anki"), QJsonObject{
        {QStringLiteral("notebookId"), notebookId},
        {QStringLiteral("path"), path},
    }, [this, path](const QJsonValue &, const QString &error) {
        setStatus(error.isEmpty() ? tr("Exported %1").arg(path) : error);
    }, this);
}

void AppController::exportCurrentEntry(const QString &path)
{
    const QString dictId = currentDictionaryId();
    if (path.isEmpty() || m_currentWord.isEmpty() || dictId.isEmpty()) {
        setStatus(tr("Look up a word before you export it."));
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
    }, this);
}

QString AppController::localFile(const QUrl &url) const
{
    return url.isLocalFile() ? url.toLocalFile() : url.toString();
}

void AppController::loadSettings()
{
    if (!m_ready) {
        return;
    }
    m_api->get(QStringLiteral("/preferences"), [this](const QJsonValue &data,
                                                      const QString &error) {
        if (!error.isEmpty() || !data.isObject()) {
            return;
        }
        const QJsonObject preferences = data.toObject();
        const QJsonValue allowValue =
            preferences.value(QStringLiteral("dictionaryallowsymlinks"));
        m_allowSymlinks = allowValue.isBool()
            ? allowValue.toBool()
            : allowValue.toString(QStringLiteral("true"))
                  .compare(QStringLiteral("false"), Qt::CaseInsensitive) != 0;
        const QString groupLimit =
            preferenceString(preferences, QStringLiteral("dictionarygroupmax"),
                             QStringLiteral("10"));
        m_groupLimit = std::clamp(groupLimit.toInt(), 1, 100);
        emit settingsChanged();
    }, this);
    refreshEcdictStatus();
}

void AppController::saveSettings(bool allowSymlinks, int groupLimit)
{
    m_allowSymlinks = allowSymlinks;
    m_groupLimit = std::clamp(groupLimit, 1, 100);
    const QJsonObject preferences{
        {QStringLiteral("dictionaryallowsymlinks"),
         m_allowSymlinks ? QStringLiteral("true") : QStringLiteral("false")},
        {QStringLiteral("dictionarygroupmax"), QString::number(m_groupLimit)},
    };
    m_api->post(QStringLiteral("/preferences"), preferences,
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            showError(tr("Settings"), error);
            return;
        }
        emit settingsChanged();
        setStatus(tr("Settings saved."));
    }, this);
}

void AppController::refreshEcdictStatus()
{
    m_api->get(QStringLiteral("/ecdict/status"), [this](const QJsonValue &data,
                                                        const QString &error) {
        if (!error.isEmpty() || !data.isObject()) {
            m_ecdictStatus = tr("The ECDICT status is not available.");
            m_ecdictFullInstalled = false;
            emit ecdictStatusChanged();
            return;
        }
        updateEcdictStatus(data.toObject());
    }, this);
}

void AppController::updateEcdictStatus(const QJsonObject &status)
{
    const QString edition = status.value(QStringLiteral("edition")).toString();
    const int count = status.value(QStringLiteral("entryCount")).toInt();
    m_ecdictFullInstalled = edition == QStringLiteral("full");
    if (m_ecdictFullInstalled) {
        m_ecdictStatus = tr("Full edition: %1 entries.")
                             .arg(QLocale().toString(count));
    } else {
        m_ecdictStatus = tr("Compact edition: %1 entries.")
                             .arg(QLocale().toString(count));
    }
    emit ecdictStatusChanged();
}

void AppController::installEcdict()
{
    if (m_ecdictBusy || m_ecdictFullInstalled) {
        return;
    }
    m_ecdictBusy = true;
    m_ecdictStatus = tr("Downloading and installing the full ECDICT data...");
    emit ecdictStatusChanged();
    m_api->post(QStringLiteral("/ecdict/install"), {},
                [this](const QJsonValue &data, const QString &error) {
        m_ecdictBusy = false;
        if (!error.isEmpty()) {
            m_ecdictStatus = tr("The full ECDICT installation failed.");
            emit ecdictStatusChanged();
            showError(tr("ECDICT"), error);
            refreshEcdictStatus();
            return;
        }
        updateEcdictStatus(data.toObject());
        refreshDictionaries();
    }, this);
}

void AppController::setCssBusy(bool busy)
{
    if (m_cssBusy == busy) {
        return;
    }
    m_cssBusy = busy;
    emit cssBusyChanged();
}

void AppController::setCssState(const QString &dictId, const QString &dictName,
                                const QString &text, const QString &status)
{
    m_cssDictId = dictId;
    m_cssDictName = dictName;
    m_cssText = text;
    m_cssStatus = status;
    emit cssStateChanged();
    emit cssTextChanged();
    emit cssStatusChanged();
}

bool AppController::prepareCssEditor()
{
    const QString dictId = currentDictionaryId();
    if (dictId.isEmpty()) {
        setStatus(tr("Select a dictionary first."));
        return false;
    }

    const QString dictName = dictionaryName(dictId);
    m_cssBaseCss.clear();
    setCssState(dictId, dictName, QString(), tr("Loading CSS..."));
    setCssBusy(true);

    QUrlQuery query;
    query.addQueryItem(QStringLiteral("dictId"), dictId);
    query.addQueryItem(QStringLiteral("word"), m_currentWord);
    const QString path = QStringLiteral("/css?") + query.query(QUrl::FullyEncoded);
    m_api->get(path, [this, dictId, dictName](const QJsonValue &data, const QString &error) {
        if (m_cssDictId != dictId) {
            return;
        }
        setCssBusy(false);
        if (!error.isEmpty() || !data.isObject()) {
            setCssState(dictId, dictName, QString(),
                        error.isEmpty() ? tr("Cannot load the dictionary CSS.") : error);
            return;
        }
        const QJsonObject payload = data.toObject();
        const QString saved = payload.value(QStringLiteral("saved")).toString();
        m_cssBaseCss = payload.value(QStringLiteral("base")).toString();
        setCssState(dictId, dictName,
                    saved.isEmpty() ? m_cssBaseCss : saved,
                    saved.isEmpty()
                        ? tr("The editor shows the dictionary CSS. Changes become an override.")
                        : tr("The editor shows the saved override."));
    }, this);
    return true;
}

void AppController::previewCss(const QString &css)
{
    emit applyCssRequested(css);
}

void AppController::saveCss(const QString &css)
{
    if (m_cssDictId.isEmpty() || m_cssBusy) {
        return;
    }
    setCssBusy(true);
    m_api->post(QStringLiteral("/css"), QJsonObject{
        {QStringLiteral("dictId"), m_cssDictId},
        {QStringLiteral("css"), css},
    }, [this, css](const QJsonValue &, const QString &error) {
        setCssBusy(false);
        if (!error.isEmpty()) {
            m_cssStatus = error;
            emit cssStatusChanged();
            return;
        }
        m_cssText = css;
        m_cssStatus = tr("CSS saved.");
        emit cssTextChanged();
        emit cssStatusChanged();
        emit applyCssRequested(css);
    }, this);
}

void AppController::removeCssOverride()
{
    if (m_cssDictId.isEmpty() || m_cssBusy) {
        return;
    }
    setCssBusy(true);
    m_api->post(QStringLiteral("/css"), QJsonObject{
        {QStringLiteral("dictId"), m_cssDictId},
        {QStringLiteral("css"), QString()},
    }, [this](const QJsonValue &, const QString &error) {
        setCssBusy(false);
        if (!error.isEmpty()) {
            m_cssStatus = error;
            emit cssStatusChanged();
            return;
        }
        m_cssText = m_cssBaseCss;
        m_cssStatus = tr("The dictionary CSS override was removed.");
        emit cssTextChanged();
        emit cssStatusChanged();
        emit applyCssRequested(m_cssBaseCss);
    }, this);
}

void AppController::restoreBaseCss()
{
    m_cssText = m_cssBaseCss;
    m_cssStatus = tr("The dictionary CSS was restored. Save to keep this result.");
    emit cssTextChanged();
    emit cssStatusChanged();
    emit applyCssRequested(m_cssBaseCss);
}

void AppController::finishCssEditor()
{
    emit reloadRequested();
}

void AppController::setCurrentDefinitionIndex(int index)
{
    const int clamped = m_definitions.isEmpty()
        ? -1
        : std::clamp(index, 0, static_cast<int>(m_definitions.size()) - 1);
    if (m_currentDefinitionIndex == clamped) {
        return;
    }
    m_currentDefinitionIndex = clamped;
    emit currentDefinitionIndexChanged();
}

void AppController::savePreferences(const QJsonObject &patch)
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
    }, this);
}
