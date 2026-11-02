// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "bookmarks_dialog.h"

#include "api_client.h"

#include <QDateTime>
#include <QDialogButtonBox>
#include <QDir>
#include <QFileDialog>
#include <QHeaderView>
#include <QHBoxLayout>
#include <QInputDialog>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPushButton>
#include <QSignalBlocker>
#include <QSplitter>
#include <QStandardPaths>
#include <QTableWidget>
#include <QVBoxLayout>

BookmarksDialog::BookmarksDialog(ApiClient *api, QWidget *parent)
    : QDialog(parent)
    , m_api(api)
{
    setWindowTitle(tr("Vocabulary notebooks"));
    resize(820, 540);
    buildUi();
    loadNotebooks();
}

void BookmarksDialog::buildUi()
{
    auto *splitter = new QSplitter(Qt::Horizontal, this);

    auto *left = new QWidget(splitter);
    auto *leftLayout = new QVBoxLayout(left);
    leftLayout->setContentsMargins(0, 0, 0, 0);
    leftLayout->addWidget(new QLabel(tr("Notebooks"), left));

    m_notebooks = new QListWidget(left);
    leftLayout->addWidget(m_notebooks, 1);

    auto *newButton = new QPushButton(tr("New"), left);
    m_renameButton = new QPushButton(tr("Rename"), left);
    m_deleteButton = new QPushButton(tr("Delete"), left);
    m_defaultButton = new QPushButton(tr("Set default"), left);
    auto *notebookButtons = new QHBoxLayout;
    notebookButtons->addWidget(newButton);
    notebookButtons->addWidget(m_renameButton);
    notebookButtons->addWidget(m_deleteButton);
    notebookButtons->addWidget(m_defaultButton);
    leftLayout->addLayout(notebookButtons);

    auto *right = new QWidget(splitter);
    auto *rightLayout = new QVBoxLayout(right);
    rightLayout->setContentsMargins(0, 0, 0, 0);

    m_table = new QTableWidget(right);
    m_table->setColumnCount(3);
    m_table->setHorizontalHeaderLabels({tr("Word"), tr("Dictionary"), tr("Saved")});
    m_table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    m_table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    m_table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    m_table->verticalHeader()->setVisible(false);
    m_table->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_table->setSelectionMode(QAbstractItemView::SingleSelection);
    m_table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    m_table->setAlternatingRowColors(true);
    rightLayout->addWidget(m_table, 1);

    auto *rightButtons = new QHBoxLayout;
    auto *openButton = new QPushButton(tr("Open"), right);
    auto *removeButton = new QPushButton(tr("Remove"), right);
    auto *exportButton = new QPushButton(tr("Export Anki..."), right);
    rightButtons->addWidget(openButton);
    rightButtons->addWidget(removeButton);
    rightButtons->addStretch();
    rightButtons->addWidget(exportButton);
    rightLayout->addLayout(rightButtons);

    splitter->addWidget(left);
    splitter->addWidget(right);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({230, 580});

    m_status = new QLabel(this);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    auto *layout = new QVBoxLayout(this);
    layout->addWidget(splitter, 1);
    layout->addWidget(m_status);
    layout->addWidget(buttons);

    connect(newButton, &QPushButton::clicked, this, &BookmarksDialog::addNotebook);
    connect(m_renameButton, &QPushButton::clicked, this, &BookmarksDialog::renameNotebook);
    connect(m_deleteButton, &QPushButton::clicked, this, &BookmarksDialog::deleteNotebook);
    connect(m_defaultButton, &QPushButton::clicked, this, &BookmarksDialog::setDefaultNotebook);
    connect(removeButton, &QPushButton::clicked, this, &BookmarksDialog::removeBookmark);
    connect(exportButton, &QPushButton::clicked, this, &BookmarksDialog::exportAnki);
    connect(openButton, &QPushButton::clicked, this, &BookmarksDialog::openSelectedBookmark);
    connect(m_table, &QTableWidget::cellDoubleClicked,
            this, [this](int, int) { openSelectedBookmark(); });
    connect(m_notebooks, &QListWidget::itemSelectionChanged, this, [this]() {
        updateNotebookActions();
        rebuildTable();
    });
    updateNotebookActions();
}

void BookmarksDialog::loadNotebooks()
{
    m_api->get(QStringLiteral("/notebooks"), [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isArray()) {
            showStatus(error.isEmpty() ? tr("Cannot load notebooks.") : error);
            return;
        }
        const QString wanted = selectedNotebookId();
        m_notebooks->clear();
        auto *all = new QListWidgetItem(tr("All words"), m_notebooks);
        all->setData(Qt::UserRole, QString());

        QListWidgetItem *defaultItem = nullptr;
        QListWidgetItem *wantedItem = nullptr;
        for (const QJsonValue &value : data.toArray()) {
            const QJsonObject notebook = value.toObject();
            const bool isDefault = notebook.value(QStringLiteral("is_default")).toBool();
            const QString name = notebook.value(QStringLiteral("name")).toString();
            auto *item = new QListWidgetItem(
                isDefault ? tr("%1 (default)").arg(name) : name, m_notebooks);
            item->setData(Qt::UserRole, notebook.value(QStringLiteral("id")).toString());
            if (isDefault) {
                defaultItem = item;
            }
            if (item->data(Qt::UserRole).toString() == wanted) {
                wantedItem = item;
            }
        }
        if (wantedItem) {
            m_notebooks->setCurrentItem(wantedItem);
        } else if (defaultItem) {
            m_notebooks->setCurrentItem(defaultItem);
        } else if (m_notebooks->count() > 0) {
            m_notebooks->setCurrentRow(0);
        }
        updateNotebookActions();
        loadBookmarks();
    }, this);
}

void BookmarksDialog::loadBookmarks()
{
    m_api->get(QStringLiteral("/bookmarks"), [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isArray()) {
            showStatus(error.isEmpty() ? tr("Cannot load saved words.") : error);
            return;
        }
        m_bookmarks = data.toArray();
        rebuildTable();
    }, this);
}

void BookmarksDialog::rebuildTable()
{
    const QString notebookId = selectedNotebookId();
    m_table->setRowCount(0);
    for (const QJsonValue &value : m_bookmarks) {
        const QJsonObject bookmark = value.toObject();
        if (!notebookId.isEmpty()
            && bookmark.value(QStringLiteral("notebook_id")).toString() != notebookId) {
            continue;
        }
        const int row = m_table->rowCount();
        m_table->insertRow(row);

        auto *wordItem = new QTableWidgetItem(
            bookmark.value(QStringLiteral("word")).toString());
        wordItem->setData(Qt::UserRole, bookmark.value(QStringLiteral("word")).toString());
        wordItem->setData(Qt::UserRole + 1, bookmark.value(QStringLiteral("dict_id")).toString());
        wordItem->setData(Qt::UserRole + 2,
                          bookmark.value(QStringLiteral("notebook_id")).toString());
        m_table->setItem(row, 0, wordItem);
        m_table->setItem(row, 1, new QTableWidgetItem(
            bookmark.value(QStringLiteral("dict_name")).toString()));

        const qint64 savedAt =
            static_cast<qint64>(bookmark.value(QStringLiteral("saved_at")).toDouble());
        m_table->setItem(row, 2, new QTableWidgetItem(
            QDateTime::fromSecsSinceEpoch(savedAt).toString(QStringLiteral("yyyy-MM-dd HH:mm"))));
    }
    showStatus(tr("%1 saved word(s)").arg(m_table->rowCount()));
}

QString BookmarksDialog::selectedNotebookId() const
{
    return m_notebooks->currentItem()
        ? m_notebooks->currentItem()->data(Qt::UserRole).toString()
        : QString();
}

void BookmarksDialog::updateNotebookActions()
{
    const bool hasNotebook = !selectedNotebookId().isEmpty();
    m_renameButton->setEnabled(hasNotebook);
    m_deleteButton->setEnabled(hasNotebook);
    m_defaultButton->setEnabled(hasNotebook);
}

void BookmarksDialog::addNotebook()
{
    bool ok = false;
    const QString name = QInputDialog::getText(
        this, tr("New notebook"), tr("Name"), QLineEdit::Normal,
        tr("Vocabulary"), &ok);
    if (!ok || name.trimmed().isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/create"),
                QJsonObject{{QStringLiteral("name"), name.trimmed()}},
                [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty()) {
            showStatus(error);
            return;
        }
        Q_UNUSED(data);
        loadNotebooks();
    }, this);
}

void BookmarksDialog::renameNotebook()
{
    QListWidgetItem *item = m_notebooks->currentItem();
    if (!item || selectedNotebookId().isEmpty()) {
        return;
    }
    bool ok = false;
    QString currentName = item->text();
    currentName.remove(tr(" (default)"));
    const QString name = QInputDialog::getText(
        this, tr("Rename notebook"), tr("Name"), QLineEdit::Normal,
        currentName, &ok);
    if (!ok || name.trimmed().isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/rename"), QJsonObject{
        {QStringLiteral("id"), selectedNotebookId()},
        {QStringLiteral("name"), name.trimmed()},
    }, [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            showStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void BookmarksDialog::deleteNotebook()
{
    const QString id = selectedNotebookId();
    if (id.isEmpty()) {
        return;
    }
    if (QMessageBox::question(this, tr("Delete notebook"),
                              tr("Move all saved words to the default notebook and delete this notebook?"))
        != QMessageBox::Yes) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/delete"), QJsonObject{{QStringLiteral("id"), id}},
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            showStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void BookmarksDialog::setDefaultNotebook()
{
    const QString id = selectedNotebookId();
    if (id.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/notebooks/default"), QJsonObject{{QStringLiteral("id"), id}},
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            showStatus(error);
            return;
        }
        loadNotebooks();
    }, this);
}

void BookmarksDialog::removeBookmark()
{
    const int row = m_table->currentRow();
    if (row < 0) {
        return;
    }
    QTableWidgetItem *item = m_table->item(row, 0);
    m_api->post(QStringLiteral("/bookmarks/remove"), QJsonObject{
        {QStringLiteral("word"), item->data(Qt::UserRole).toString()},
        {QStringLiteral("dictId"), item->data(Qt::UserRole + 1).toString()},
        {QStringLiteral("notebookId"), item->data(Qt::UserRole + 2).toString()},
    }, [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            showStatus(error);
            return;
        }
        loadBookmarks();
    }, this);
}

void BookmarksDialog::exportAnki()
{
    const QString suggested = QDir(QStandardPaths::writableLocation(
        QStandardPaths::DocumentsLocation)).filePath(QStringLiteral("medict.apkg"));
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Export Anki package"), suggested, tr("Anki packages (*.apkg)"));
    if (path.isEmpty()) {
        return;
    }
    m_api->post(QStringLiteral("/anki"), QJsonObject{
        {QStringLiteral("notebookId"), selectedNotebookId()},
        {QStringLiteral("path"), path},
    }, [this, path](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            showStatus(error);
            return;
        }
        showStatus(tr("Exported %1").arg(path));
    }, this);
}

void BookmarksDialog::openSelectedBookmark()
{
    const int row = m_table->currentRow();
    if (row < 0) {
        return;
    }
    QTableWidgetItem *item = m_table->item(row, 0);
    const QString word = item->data(Qt::UserRole).toString();
    const QString dictId = item->data(Qt::UserRole + 1).toString();
    if (word.isEmpty()) {
        return;
    }
    emit activateWord(word, dictId);
    accept();
}

void BookmarksDialog::showStatus(const QString &message)
{
    m_status->setText(message);
}
