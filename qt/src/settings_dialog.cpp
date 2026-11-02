// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "settings_dialog.h"

#include "api_client.h"

#include <QApplication>
#include <QCheckBox>
#include <QDesktopServices>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QFont>
#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QLocale>
#include <QMessageBox>
#include <QProgressBar>
#include <QPushButton>
#include <QSpinBox>
#include <QTabWidget>
#include <QUrl>
#include <QVBoxLayout>

SettingsDialog::SettingsDialog(ApiClient *api, const QString &baseDictDir, QWidget *parent)
    : QDialog(parent)
    , m_api(api)
    , m_baseDictDir(baseDictDir)
{
    setWindowTitle(tr("Settings"));
    resize(650, 440);
    buildUi();
    loadPreferences();
    loadEcdictStatus();
}

void SettingsDialog::buildUi()
{
    auto *tabs = new QTabWidget(this);

    auto *dictionaryTab = new QWidget(tabs);
    auto *dictionaryLayout = new QVBoxLayout(dictionaryTab);
    auto *dictionaryForm = new QFormLayout;

    auto *pathRow = new QWidget(dictionaryTab);
    auto *pathLayout = new QHBoxLayout(pathRow);
    pathLayout->setContentsMargins(0, 0, 0, 0);
    m_dictPath = new QLineEdit(m_baseDictDir, pathRow);
    m_dictPath->setReadOnly(true);
    auto *openFolderButton = new QPushButton(tr("Open folder"), pathRow);
    pathLayout->addWidget(m_dictPath, 1);
    pathLayout->addWidget(openFolderButton);
    dictionaryForm->addRow(tr("Dictionary folder"), pathRow);
    dictionaryLayout->addLayout(dictionaryForm);

    auto *reloadButton = new QPushButton(tr("Reload dictionaries"), dictionaryTab);
    dictionaryLayout->addWidget(reloadButton);
    dictionaryLayout->addSpacing(16);

    auto *ecdictTitle = new QLabel(tr("Full English-Chinese dictionary"), dictionaryTab);
    QFont titleFont = ecdictTitle->font();
    titleFont.setBold(true);
    ecdictTitle->setFont(titleFont);
    dictionaryLayout->addWidget(ecdictTitle);

    m_ecdictStatus = new QLabel(tr("Reading dictionary status..."), dictionaryTab);
    m_ecdictStatus->setWordWrap(true);
    dictionaryLayout->addWidget(m_ecdictStatus);

    auto *installRow = new QHBoxLayout;
    m_installButton = new QPushButton(tr("Download full ECDICT"), dictionaryTab);
    m_progress = new QProgressBar(dictionaryTab);
    m_progress->setRange(0, 0);
    m_progress->setVisible(false);
    installRow->addWidget(m_installButton);
    installRow->addWidget(m_progress, 1);
    dictionaryLayout->addLayout(installRow);
    dictionaryLayout->addStretch();
    tabs->addTab(dictionaryTab, tr("Dictionaries"));

    auto *preferencesTab = new QWidget(tabs);
    auto *preferencesLayout = new QVBoxLayout(preferencesTab);
    auto *preferencesForm = new QFormLayout;
    m_allowSymlinks = new QCheckBox(tr("Load dictionaries through symbolic links"),
                                    preferencesTab);
    m_groupLimit = new QSpinBox(preferencesTab);
    m_groupLimit->setRange(1, 100);
    preferencesForm->addRow(tr("Load rules"), m_allowSymlinks);
    preferencesForm->addRow(tr("Maximum dictionaries per group"), m_groupLimit);
    preferencesLayout->addLayout(preferencesForm);
    preferencesLayout->addWidget(new QLabel(tr("The interface follows the system theme."),
                                             preferencesTab));
    preferencesLayout->addStretch();
    tabs->addTab(preferencesTab, tr("Preferences"));

    auto *aboutTab = new QWidget(tabs);
    auto *aboutLayout = new QVBoxLayout(aboutTab);
    auto *about = new QLabel(
        tr("<h3>Medict %1</h3>"
           "<p>Native Qt desktop client with a Go dictionary engine.</p>"
           "<p>Qt %2</p>"
           "<p>License: GNU GPL v3 or later.</p>")
            .arg(QApplication::applicationVersion(), QString::fromLatin1(qVersion())),
        aboutTab);
    about->setWordWrap(true);
    about->setOpenExternalLinks(true);
    aboutLayout->addWidget(about);
    aboutLayout->addStretch();
    tabs->addTab(aboutTab, tr("About"));

    auto *buttons = new QDialogButtonBox(
        QDialogButtonBox::Save | QDialogButtonBox::Cancel, this);

    auto *layout = new QVBoxLayout(this);
    layout->addWidget(tabs, 1);
    layout->addWidget(buttons);

    connect(openFolderButton, &QPushButton::clicked, this, [this]() {
        QDesktopServices::openUrl(QUrl::fromLocalFile(m_baseDictDir));
    });
    connect(reloadButton, &QPushButton::clicked, this, [this]() {
        emit dictionariesChanged();
    });
    connect(m_installButton, &QPushButton::clicked, this, &SettingsDialog::installEcdict);
    connect(buttons, &QDialogButtonBox::accepted, this, &SettingsDialog::savePreferences);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
}

void SettingsDialog::loadPreferences()
{
    m_api->get(QStringLiteral("/preferences"), [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isObject()) {
            return;
        }
        const QJsonObject preferences = data.toObject();
        const QJsonValue allowValue =
            preferences.value(QStringLiteral("dictionaryallowsymlinks"));
        const bool allow = allowValue.isBool()
            ? allowValue.toBool()
            : allowValue.toString(QStringLiteral("true"))
                  .compare(QStringLiteral("false"), Qt::CaseInsensitive) != 0;
        m_allowSymlinks->setChecked(allow);
        const QString groupLimit =
            preferences.value(QStringLiteral("dictionarygroupmax")).toString();
        m_groupLimit->setValue(groupLimit.isEmpty() ? 10 : groupLimit.toInt());
    }, this);
}

void SettingsDialog::loadEcdictStatus()
{
    m_api->get(QStringLiteral("/ecdict/status"), [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isObject()) {
            m_ecdictStatus->setText(tr("The ECDICT status is not available."));
            return;
        }
        const QJsonObject status = data.toObject();
        const QString edition = status.value(QStringLiteral("edition")).toString();
        const int count = status.value(QStringLiteral("entryCount")).toInt();
        if (edition == QStringLiteral("full")) {
            m_ecdictStatus->setText(
                tr("Full edition: %1 entries.").arg(QLocale().toString(count)));
            m_installButton->setEnabled(false);
            m_installButton->setText(tr("Full edition installed"));
        } else {
            m_ecdictStatus->setText(
                tr("Compact edition: %1 entries.").arg(QLocale().toString(count)));
            m_installButton->setEnabled(true);
            m_installButton->setText(tr("Download full ECDICT"));
        }
    }, this);
}

void SettingsDialog::savePreferences()
{
    const QJsonObject preferences{
        {QStringLiteral("dictionaryallowsymlinks"),
         m_allowSymlinks->isChecked() ? QStringLiteral("true") : QStringLiteral("false")},
        {QStringLiteral("dictionarygroupmax"), QString::number(m_groupLimit->value())},
    };
    m_api->post(QStringLiteral("/preferences"), preferences,
                [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            QMessageBox::warning(this, tr("Settings"), error);
            return;
        }
        accept();
    }, this);
}

void SettingsDialog::installEcdict()
{
    setEcdictBusy(true);
    m_ecdictStatus->setText(tr("Downloading and installing the full ECDICT data..."));
    m_api->post(QStringLiteral("/ecdict/install"), {},
                [this](const QJsonValue &data, const QString &error) {
        setEcdictBusy(false);
        if (!error.isEmpty()) {
            m_ecdictStatus->setText(tr("The full ECDICT installation failed."));
            QMessageBox::warning(this, tr("ECDICT"), error);
            loadEcdictStatus();
            return;
        }
        const int count = data.toObject().value(QStringLiteral("entryCount")).toInt();
        m_ecdictStatus->setText(
            tr("Full edition installed: %1 entries.").arg(QLocale().toString(count)));
        m_installButton->setEnabled(false);
        m_installButton->setText(tr("Full edition installed"));
        emit dictionariesChanged();
    }, this);
}

void SettingsDialog::setEcdictBusy(bool busy)
{
    m_installButton->setEnabled(!busy);
    m_progress->setVisible(busy);
}
