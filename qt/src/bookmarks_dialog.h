// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <QDialog>
#include <QJsonArray>

class ApiClient;
class QLabel;
class QListWidget;
class QTableWidget;
class QPushButton;

class BookmarksDialog final : public QDialog
{
    Q_OBJECT

public:
    explicit BookmarksDialog(ApiClient *api, QWidget *parent = nullptr);

signals:
    void activateWord(const QString &word, const QString &dictId);

private:
    void buildUi();
    void loadNotebooks();
    void loadBookmarks();
    void rebuildTable();
    void updateNotebookActions();
    QString selectedNotebookId() const;
    void addNotebook();
    void renameNotebook();
    void deleteNotebook();
    void setDefaultNotebook();
    void removeBookmark();
    void exportAnki();
    void openSelectedBookmark();
    void showStatus(const QString &message);

    ApiClient *m_api = nullptr;
    QListWidget *m_notebooks = nullptr;
    QTableWidget *m_table = nullptr;
    QLabel *m_status = nullptr;
    QPushButton *m_renameButton = nullptr;
    QPushButton *m_deleteButton = nullptr;
    QPushButton *m_defaultButton = nullptr;
    QJsonArray m_bookmarks;
};

