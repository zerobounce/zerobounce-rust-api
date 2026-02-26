//! Validation status and sub-status enums for API responses.
//!
//! Use for type-safe comparison. Unknown/future API values map to `Unknown(String)`.

use std::str::FromStr;

/// Validation status values returned by the API (validate, validate_batch).
/// Parse from string: `let s: ZBValidateStatus = response.status.parse().unwrap_or(ZBValidateStatus::Unknown(response.status.clone()));`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZBValidateStatus {
    None,
    Valid,
    Invalid,
    CatchAll,
    Unknown,
    Spamtrap,
    Abuse,
    DoNotMail,
    /// Unknown or future API value (backward compatible)
    UnknownValue(String),
}

impl FromStr for ZBValidateStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "" => ZBValidateStatus::None,
            "valid" => ZBValidateStatus::Valid,
            "invalid" => ZBValidateStatus::Invalid,
            "catch-all" => ZBValidateStatus::CatchAll,
            "unknown" => ZBValidateStatus::Unknown,
            "spamtrap" => ZBValidateStatus::Spamtrap,
            "abuse" => ZBValidateStatus::Abuse,
            "do_not_mail" => ZBValidateStatus::DoNotMail,
            other => ZBValidateStatus::UnknownValue(other.to_string()),
        })
    }
}

/// Validation sub-status values returned by the API (validate, validate_batch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZBValidateSubStatus {
    None,
    AntispamSystem,
    Greylisted,
    MailServerTemporaryError,
    ForcibleDisconnect,
    MailServerDidNotRespond,
    TimeoutExceeded,
    FailedSmtpConnection,
    MailboxQuotaExceeded,
    ExceptionOccurred,
    PossibleTrap,
    RoleBased,
    GlobalSuppression,
    MailboxNotFound,
    NoDnsEntries,
    FailedSyntaxCheck,
    PossibleTypo,
    UnroutableIpAddress,
    LeadingPeriodRemoved,
    DoesNotAcceptMail,
    AliasAddress,
    RoleBasedCatchAll,
    Disposable,
    Toxic,
    Alternate,
    MxForward,
    Blocked,
    Allowed,
    AcceptAll,
    RoleBasedAcceptAll,
    Gold,
    /// Unknown or future API value (backward compatible)
    UnknownValue(String),
}

impl FromStr for ZBValidateSubStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "" => ZBValidateSubStatus::None,
            "antispam_system" => ZBValidateSubStatus::AntispamSystem,
            "greylisted" => ZBValidateSubStatus::Greylisted,
            "mail_server_temporary_error" => ZBValidateSubStatus::MailServerTemporaryError,
            "forcible_disconnect" => ZBValidateSubStatus::ForcibleDisconnect,
            "mail_server_did_not_respond" => ZBValidateSubStatus::MailServerDidNotRespond,
            "timeout_exceeded" => ZBValidateSubStatus::TimeoutExceeded,
            "failed_smtp_connection" => ZBValidateSubStatus::FailedSmtpConnection,
            "mailbox_quota_exceeded" => ZBValidateSubStatus::MailboxQuotaExceeded,
            "exception_occurred" => ZBValidateSubStatus::ExceptionOccurred,
            "possible_trap" => ZBValidateSubStatus::PossibleTrap,
            "role_based" => ZBValidateSubStatus::RoleBased,
            "global_suppression" => ZBValidateSubStatus::GlobalSuppression,
            "mailbox_not_found" => ZBValidateSubStatus::MailboxNotFound,
            "no_dns_entries" => ZBValidateSubStatus::NoDnsEntries,
            "failed_syntax_check" => ZBValidateSubStatus::FailedSyntaxCheck,
            "possible_typo" => ZBValidateSubStatus::PossibleTypo,
            "unroutable_ip_address" => ZBValidateSubStatus::UnroutableIpAddress,
            "leading_period_removed" => ZBValidateSubStatus::LeadingPeriodRemoved,
            "does_not_accept_mail" => ZBValidateSubStatus::DoesNotAcceptMail,
            "alias_address" => ZBValidateSubStatus::AliasAddress,
            "role_based_catch_all" => ZBValidateSubStatus::RoleBasedCatchAll,
            "disposable" => ZBValidateSubStatus::Disposable,
            "toxic" => ZBValidateSubStatus::Toxic,
            "alternate" => ZBValidateSubStatus::Alternate,
            "mx_forward" => ZBValidateSubStatus::MxForward,
            "blocked" => ZBValidateSubStatus::Blocked,
            "allowed" => ZBValidateSubStatus::Allowed,
            "accept_all" => ZBValidateSubStatus::AcceptAll,
            "role_based_accept_all" => ZBValidateSubStatus::RoleBasedAcceptAll,
            "gold" => ZBValidateSubStatus::Gold,
            other => ZBValidateSubStatus::UnknownValue(other.to_string()),
        })
    }
}
