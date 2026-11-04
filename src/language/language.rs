use std::fmt;
use thiserror::Error;

/// Language for the fixed labels printed on the slip ("Payable to", "Currency",
/// "Amount", ...) - not the language of any user-supplied text such as the
/// unstructured message, which is passed through unchanged.
///
/// Defaults to [`Language::De`], matching the rest of this crate's tooling
/// (the web frontend defaults to German too).
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Language {
    #[default]
    De,
    Fr,
    It,
    En,
    /// Romansh, a correspondence language since IG QR-bill v2.4 (3.2).
    Rm,
}
impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::De => write!(f, "Deutsch"),
            Language::Fr => write!(f, "Français"),
            Language::It => write!(f, "Italiano"),
            Language::En => write!(f, "English"),
            Language::Rm => write!(f, "Rumantsch"),
        }
    }
}



#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum LabelKey {
    PaymentPart,
    AccountPayableTo,
    Reference,
    AdditionalInformation,
    Currency,
    Amount,
    Receipt,
    AcceptancePoint,
    SeparateBeforePayingIn,
    PayableBy,
    PayableByNameAddress,
    InFavourOf,
    BillingInformation,
    AlternativeProcedures,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Error)]
pub enum LanguageError {
    #[error("No Label Found for key: {0:?}")]
    LabelNotFound(LabelKey),
}

pub struct LabelEntry {
    pub key: LabelKey,
    pub lang: Language,
    pub text: &'static str,
}

const LABELS: &[(LabelKey, Language, &str)] = &[
    (LabelKey::PaymentPart, Language::De, "Zahlteil"),
    (LabelKey::PaymentPart, Language::Fr, "Section paiement"),
    (LabelKey::PaymentPart, Language::It, "Sezione pagamento"),
    (LabelKey::PaymentPart, Language::En, "Payment part"),
    (LabelKey::PaymentPart, Language::Rm, "Part da pajament"),

    (LabelKey::AccountPayableTo, Language::De, "Konto / Zahlbar an"),
    (LabelKey::AccountPayableTo, Language::Fr, "Compte / Payable à"),
    (LabelKey::AccountPayableTo, Language::It, "Conto / Pagabile a"),
    (LabelKey::AccountPayableTo, Language::En, "Account / Payable to"),
    (LabelKey::AccountPayableTo, Language::Rm, "Conto / Da pajar a"),

    (LabelKey::Reference, Language::De, "Referenz"),
    (LabelKey::Reference, Language::Fr, "Référence"),
    (LabelKey::Reference, Language::It, "Riferimento"),
    (LabelKey::Reference, Language::En, "Reference"),
    (LabelKey::Reference, Language::Rm, "Referenza"),

    (LabelKey::AdditionalInformation, Language::De, "Zusätzliche Informationen"),
    (LabelKey::AdditionalInformation, Language::Fr, "Informations supplémentaires"),
    (LabelKey::AdditionalInformation, Language::It, "Informazioni supplementari"),
    (LabelKey::AdditionalInformation, Language::En, "Additional information"),
    (LabelKey::AdditionalInformation, Language::Rm, "Infurmaziuns supplementaras"),

    (LabelKey::Currency, Language::De, "Währung"),
    (LabelKey::Currency, Language::Fr, "Monnaie"),
    (LabelKey::Currency, Language::It, "Valuta"),
    (LabelKey::Currency, Language::En, "Currency"),
    (LabelKey::Currency, Language::Rm, "Valuta"),

    (LabelKey::Amount, Language::De, "Betrag"),
    (LabelKey::Amount, Language::Fr, "Montant"),
    (LabelKey::Amount, Language::It, "Importo"),
    (LabelKey::Amount, Language::En, "Amount"),
    (LabelKey::Amount, Language::Rm, "Import"),

    (LabelKey::Receipt, Language::De, "Empfangsschein"),
    (LabelKey::Receipt, Language::Fr, "Récépissé"),
    (LabelKey::Receipt, Language::It, "Ricevuta"),
    (LabelKey::Receipt, Language::En, "Receipt"),
    (LabelKey::Receipt, Language::Rm, "Quittanza"),

    (LabelKey::AcceptancePoint, Language::De, "Annahmestelle"),
    (LabelKey::AcceptancePoint, Language::Fr, "Point de dépôt"),
    (LabelKey::AcceptancePoint, Language::It, "Punto di accettazione"),
    (LabelKey::AcceptancePoint, Language::En, "Acceptance point"),
    (LabelKey::AcceptancePoint, Language::Rm, "Post da recepziun"),

    (LabelKey::SeparateBeforePayingIn, Language::De, "Vor der Einzahlung abzutrennen"),
    (LabelKey::SeparateBeforePayingIn, Language::Fr, "A détacher avant le versement"),
    (LabelKey::SeparateBeforePayingIn, Language::It, "Da staccare prima del versamento"),
    (LabelKey::SeparateBeforePayingIn, Language::En, "Separate before paying in"),
    (LabelKey::SeparateBeforePayingIn, Language::Rm, "Da distatgar avant che pajar"),

    (LabelKey::PayableBy, Language::De, "Zahlbar durch"),
    (LabelKey::PayableBy, Language::Fr, "Payable par"),
    (LabelKey::PayableBy, Language::It, "Pagabile da"),
    (LabelKey::PayableBy, Language::En, "Payable by"),
    (LabelKey::PayableBy, Language::Rm, "Da pajar da"),

    (LabelKey::PayableByNameAddress, Language::De, "Zahlbar durch (Name/Adresse)"),
    (LabelKey::PayableByNameAddress, Language::Fr, "Payable par (nom/adresse)"),
    (LabelKey::PayableByNameAddress, Language::It, "Pagabile da (nome/indirizzo)"),
    (LabelKey::PayableByNameAddress, Language::En, "Payable by (name/address)"),
    (LabelKey::PayableByNameAddress, Language::Rm, "Da pajar da (num/adressa)"),

    (LabelKey::InFavourOf, Language::De, "Zugunsten"),
    (LabelKey::InFavourOf, Language::Fr, "En faveur de"),
    (LabelKey::InFavourOf, Language::It, "A favore di"),
    (LabelKey::InFavourOf, Language::En, "In favour of"),
    (LabelKey::InFavourOf, Language::Rm, "En favur da"),

    (LabelKey::BillingInformation, Language::De, "Rechnungsinformationen"),
    (LabelKey::BillingInformation, Language::Fr, "Informations de facture"),
    (LabelKey::BillingInformation, Language::It, "Informazioni per la fattura"),
    (LabelKey::BillingInformation, Language::En, "Billing information"),
    (LabelKey::BillingInformation, Language::Rm, "Infurmaziuns davart il quint"),

    (LabelKey::AlternativeProcedures, Language::De, "Alternative Verfahren"),
    (LabelKey::AlternativeProcedures, Language::Fr, "Procédures alternatives"),
    (LabelKey::AlternativeProcedures, Language::It, "Procedure alternative"),
    (LabelKey::AlternativeProcedures, Language::En, "Alternative procedures"),
    (LabelKey::AlternativeProcedures, Language::Rm, "Proceduras alternativas"),

];

pub fn label(key: LabelKey, lang: Language) -> Result<&'static str, LanguageError> {
    // Try requested language
    LABELS
        .iter()
        .find(|&&(k, l, _)| k == key && l == lang)
        .map(|&(_, _, val)| val)
        // Fallback to English
        .or_else(|| {
            LABELS
                .iter()
                .find(|&&(k, l, _)| k == key && l == Language::default())
                .map(|&(_, _, val)| val)
        })
        // If still not found, return an error
        .ok_or(LanguageError::LabelNotFound(key))
}

/// Exports a macro for the above
#[macro_export]
macro_rules! label {
    ($key:ident) => {
        $crate::language::label(
            $crate::language::LabelKey::$key,
            $crate::language::Language::default(),
        )
        .expect("Missing label")
    };
    ($key:ident, $lang:expr) => {
        $crate::language::label(
            $crate::language::LabelKey::$key,
            $lang,
        )
        .expect("Missing label")
    };
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_german_label() {
        assert_eq!(label(LabelKey::PaymentPart, Language::De ).unwrap(), "Zahlteil");
    }

    #[test]
    fn test_french_label() {
        assert_eq!(label(LabelKey::AcceptancePoint, Language::Fr ).unwrap(), "Point de dépôt");
    }

    #[test]
    fn test_italian_label() {
        assert_eq!(label(LabelKey::AdditionalInformation, Language::It).unwrap(), "Informazioni supplementari")
    }

    #[test]
    fn every_label_has_a_romansh_text() {
        for &(key, lang, _) in LABELS {
            if lang == Language::De {
                assert!(LABELS.iter().any(|&(k, l, _)| k == key && l == Language::Rm), "{key:?}");
            }
        }
        assert_eq!(label(LabelKey::PaymentPart, Language::Rm).unwrap(), "Part da pajament");
    }

    #[test]
    fn test_macro() {
        assert_eq!(label!(PayableByNameAddress, Language::En), "Payable by (name/address)");
    }

    #[test]
    fn test_default() {
        assert_eq!(label!(InFavourOf), "Zugunsten");
    }
}
