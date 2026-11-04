"""Generates tests/data/six/ig_qr_bill_v2_4_vectors.json.

Usage: python3 scripts/gen_six_vectors.py [output path]

Every expected payload is built from Table 8 (IG QR-bill v2.4, 4.2.2) and the
Annex A example tables - not from the library under test. Check digits are
computed here independently.
"""
import json
import sys

SPEC = "IG QR-bill v2.4"


def mod97_ok(s):
    s = s.replace(" ", "").upper()
    s = s[4:] + s[:4]
    n = "".join(str(int(c, 36)) for c in s)
    return int(n) % 97 == 1


def iban_with_check(cc, bban):
    n = "".join(str(int(c, 36)) for c in bban + cc + "00")
    return f"{cc}{98 - int(n) % 97:02d}{bban}"


MOD10 = [0, 9, 4, 6, 8, 2, 7, 1, 3, 5]


def qrr_with_check(raw26):
    carry = 0
    for d in raw26:
        carry = MOD10[(carry + int(d)) % 10]
    return raw26 + str((10 - carry) % 10)


def addr(name, street, house, plz, city, country):
    a = {"name": name, "plz": plz, "city": city, "country": country}
    if street is not None:
        a["street"] = street
    if house is not None:
        a["house_num"] = house
    return a


def addr_lines(a):
    if a is None:
        return [""] * 7
    return ["S", a["name"], a.get("street", ""), a.get("house_num", ""), a["plz"], a["city"], a["country"]]


def lines(inp, ref_type, ref, bill_info=None, alt=()):
    """Table 8 order: header, CdtrInf, UltmtCdtr (X), CcyAmt, UltmtDbtr, RmtInf, AddInf, AltPmtInf."""
    out = ["SPC", "0200", "1", inp["iban"].replace(" ", "")]
    out += addr_lines(inp["creditor_address"])
    out += [""] * 7
    out += [inp.get("amount", ""), inp["currency"]]
    out += addr_lines(inp.get("debtor_address"))
    out += [ref_type, ref, inp.get("unstructured_message", ""), "EPD"]
    if bill_info is not None or alt:
        out.append(bill_info or "")
    out += list(alt)
    return out


def bill(iban, cdtr, currency="CHF", amount=None, dbtr=None, reference=None, ustrd=None, **extra):
    b = {"iban": iban, "creditor_address": cdtr, "currency": currency}
    if amount is not None:
        b["amount"] = amount
    if dbtr is not None:
        b["debtor_address"] = dbtr
    if reference is not None:
        b["reference"] = reference
    if ustrd is not None:
        b["unstructured_message"] = ustrd
    b.update(extra)
    return b


vectors = []


def vec(id, spec, description, input, expect, known_gap=None):
    v = {"id": id, "spec": f"{SPEC}, {spec}", "description": description, "input": input, "expect": expect}
    if known_gap:
        v["known_gap"] = known_gap
    vectors.append(v)


# --- Annex A examples (German original values, Tables 17-22) ----------------
MAX = addr("Max Muster & Söhne", "Musterstrasse", "123", "8000", "Seldwyla", "CH")
SIMON = addr("Simon Muster", "Musterstrasse", "1", "8000", "Seldwyla", "CH")
SARAH = addr("Sarah Beispiel", "Musterstrasse", "1", "8000", "Seldwyla", "CH")
SWICO_A2 = "//S1/10/1234/11/201021/30/102673386/32/7.7/40/0:30"
EBILL_A2 = "eBill/B/simon.muster@example.com"

i = bill("CH6431961000004421557", MAX, amount="50.00", dbtr=SIMON,
         reference="000008207791225857421286694", ustrd="Bezahlung der Reise")
vec("ANNEX-A-1", "Annex A, Table 17", "QR-IBAN + QR reference, amount and debtor",
    i, {"qr_lines": lines(i, "QRR", "000008207791225857421286694")})

i = bill("CH4431999123000889012", MAX, amount="1949.75", dbtr=SIMON,
         reference="210000000003139471430009017", ustrd="Auftrag vom 15.10.2020",
         bill_information=SWICO_A2, alternative_schemes=[EBILL_A2, None])
vec("ANNEX-A-2", "Annex A, Table 18", "QR-IBAN + QRR with billing information and one alternative procedure",
    i, {"qr_lines": lines(i, "QRR", "210000000003139471430009017", SWICO_A2, [EBILL_A2])})

i2 = dict(i)
del i2["bill_information"]
i2["swico_bill_information"] = {
    "invoice_number": "1234", "invoice_date": "2020-10-21", "vat_number": "102673386",
    "vat_rate": 7.7, "payment_conditions": [{"discount": 0, "days": 30}],
}
vec("ANNEX-A-2-SWICO", "Annex A, Table 18 + Annex D",
    "Same as ANNEX-A-2, billing information supplied as structured S1 fields",
    i2, {"qr_lines": lines(i, "QRR", "210000000003139471430009017", SWICO_A2, [EBILL_A2])})

i = bill("CH5204835012345671000", addr("Muster Stiftung", "Postfach", None, "3001", "Bern", "CH"))
vec("ANNEX-A-3", "Annex A, Table 19", "IBAN without amount, debtor or reference (donation)",
    i, {"qr_lines": lines(i, "NON", "")})

# Table 20 prints "RF720191230100405JSH0438", whose mod 97-10 check digits are
# wrong (the remainder is 49, not 1). SIX's sample sheet Nr. 17 uses the
# correct "RF24...". Use that for the payload and keep the printed one as a
# reject vector.
assert not mod97_ok("RF720191230100405JSH0438") and mod97_ok("RF240191230100405JSH0438")
i = bill("CH5800791123000889012", addr("Muster Krankenkasse", "Musterstrasse", "12", "8000", "Seldwyla", "CH"),
         amount="211.00", dbtr=SARAH, reference="RF240191230100405JSH0438")
vec("ANNEX-A-4", "Annex A, Table 20 (check digits corrected)", "IBAN + Creditor Reference (SCOR)",
    i, {"qr_lines": lines(i, "SCOR", "RF240191230100405JSH0438")})
vec("ANNEX-A-4-AS-PRINTED", "Annex A, Table 20 / 2.12.2",
    "The reference exactly as printed in Table 20 has invalid check digits and must be rejected",
    dict(i, reference="RF720191230100405JSH0438"), "reject")

i = bill("CH5800791123000889012", addr("Max Muster & Söhne", "Musterstrasse", "123", "9490", "Vaduz", "LI"),
         amount="199.95", dbtr=SARAH, reference="RF18539007547034")
vec("ANNEX-A-5", "Annex A, Table 21", "Creditor in Liechtenstein",
    i, {"qr_lines": lines(i, "SCOR", "RF18539007547034")})

i = bill("CH5800791123000889012", MAX, amount="199.95",
         dbtr=addr("Sarah Beispiel", "Musterstrasse", "1", "78462", "Konstanz", "DE"), reference="RF18539007547034")
vec("ANNEX-A-6", "Annex A, Table 22", "Debtor outside Switzerland",
    i, {"qr_lines": lines(i, "SCOR", "RF18539007547034")})

# --- 4.4 "DO NOT USE FOR PAYMENT" -------------------------------------------
NOTIFY = addr("Muster Krankenkasse", "Musterstrasse", "12", "8000", "Seldwyla", "CH")
for lang, text in [("DE", "NICHT ZUR ZAHLUNG VERWENDEN"), ("EN", "DO NOT USE FOR PAYMENT"),
                   ("RM", "BETG DUVRAR PER IL PAJAMENT")]:
    i = bill("CH5800791123000889012", NOTIFY, amount="0.00", dbtr=SARAH, ustrd=text)
    vec(f"NOTIFY-{lang}", "4.4, Table 10",
        "Notification bill: amount must be 0.00 with the fixed 'do not use for payment' text",
        i, {"qr_lines": lines(i, "NON", "")})

# --- Line definitions that must be accepted ----------------------------------
BASE_IBAN = "CH5800791123000889012"
CDTR = addr("Muster Krankenkasse", "Musterstrasse", "12", "8000", "Seldwyla", "CH")

i = bill("CH58 0079 1123 0008 8901 2", CDTR, amount="10.00")
vec("IBAN-SPACES-STRIPPED", "4.2.2 line 4", "IBAN given with spaces is written without spaces",
    i, {"qr_lines": lines(i, "NON", "")})

for amt, why in [("0.01", "minimum amount"), ("999999999.99", "maximum amount"), ("0.10", "amount below 1.00")]:
    i = bill(BASE_IBAN, CDTR, amount=amt)
    vec(f"AMT-OK-{amt}", "4.2.2 line 19", f"Accepts {why}", i, {"qr_lines": lines(i, "NON", "")})

i = bill(BASE_IBAN, CDTR, currency="EUR", amount="10.00", reference="RF18539007547034")
vec("EUR-IBAN-SCOR", "1.1 / 4.3.2", "EUR with IBAN + SCOR is permitted", i, {"qr_lines": lines(i, "SCOR", "RF18539007547034")})
i = bill(BASE_IBAN, CDTR, currency="EUR", amount="10.00", ustrd="Rechnung 42")
vec("EUR-IBAN-NON", "1.1 / 4.3.3", "EUR with IBAN + unstructured message is permitted", i, {"qr_lines": lines(i, "NON", "")})

i = bill(BASE_IBAN, addr("Muster AG", None, None, "8000", "Seldwyla", "CH"))
vec("ADDR-NO-STREET", "4.2.2 lines 7-8", "Street and building number are optional", i, {"qr_lines": lines(i, "NON", "")})
i = bill(BASE_IBAN, addr("Muster AG", "Musterstrasse 12", None, "8000", "Seldwyla", "CH"))
vec("ADDR-HOUSE-IN-STREET", "4.3.1 footnote 2", "Building number inside the street line is tolerated",
    i, {"qr_lines": lines(i, "NON", "")})

NAME70 = "Genossenschaft für Lärmschutz, Grünflächen und Gewässer, Küsnacht ZH"
NAME70 = NAME70 + "ä" * (70 - len(NAME70))
assert len(NAME70) == 70 and len(NAME70.encode()) > 70, (len(NAME70), len(NAME70.encode()))
i = bill(BASE_IBAN, addr(NAME70, "Musterstrasse", "12", "8000", "Seldwyla", "CH"))
vec("ADDR-NAME-70-CHARS-UMLAUT", "4.2.2 line 6", "Name of exactly 70 characters (more than 70 UTF-8 bytes)",
    i, {"qr_lines": lines(i, "NON", "")})
CITY35 = "Saint-Légier-La Chiésaz, Vevey (VD)"
assert len(CITY35) == 35 and len(CITY35.encode()) > 35, len(CITY35)
i = bill(BASE_IBAN, addr("Muster AG", "Rue du Lac", "1", "1806", CITY35, "CH"))
vec("ADDR-TOWN-35-CHARS-ACCENT", "4.2.2 line 10", "Town of exactly 35 characters (more than 35 UTF-8 bytes)",
    i, {"qr_lines": lines(i, "NON", "")})

i = bill(BASE_IBAN, CDTR, ustrd="€" * 140)
vec("USTRD-140-CHARS-MULTIBYTE", "4.2.2 line 30 / 4.1.1", "Unstructured message of 140 multi-byte characters",
    i, {"qr_lines": lines(i, "NON", "")})
i = bill(BASE_IBAN, CDTR, ustrd="a" * 100, bill_information="//S1/10/" + "1" * 32)
vec("ADDINF-140-COMBINED", "4.3.3", "Ustrd + StrdBkgInf together exactly 140 characters",
    i, {"qr_lines": lines(i, "NON", "", "//S1/10/" + "1" * 32)})
i = bill(BASE_IBAN, CDTR, alternative_schemes=["eBill/B/" + "x" * 92, "Twint/" + "y" * 94])
vec("ALTPMT-2x100", "4.2.2 lines 33-34", "Two alternative procedures of 100 characters each",
    i, {"qr_lines": lines(i, "NON", "", None, ["eBill/B/" + "x" * 92, "Twint/" + "y" * 94])})
i = bill(BASE_IBAN, CDTR, ustrd="Șipot Țara € «ß» ÿ Ł ő")
vec("CHARSET-EXTENDED", "4.1.1", "Characters from Latin-1 Supplement, Latin Extended-A, Ș ș Ț ț and €",
    i, {"qr_lines": lines(i, "NON", "")})

# Creditor Reference given with spaces: either normalise or reject - never
# write a non-alphanumeric reference line.
i = bill(BASE_IBAN, CDTR, reference="RF18 5390 0754 7034")
vec("SCOR-SPACES", "4.2.2 line 29 / 2.12.2",
    "Creditor Reference typed with spaces must be written without spaces (or rejected)",
    i, {"qr_lines_or_reject": lines(i, "SCOR", "RF18539007547034")})

# 6.2: the payload must fit version 25 / ECC M. All lines at their maximum
# character count, with multi-byte text where the library counts characters.
BIG = addr("a" * 70, "a" * 70, "a" * 16, "1" * 16, "a" * 35, "CH")
i = bill("CH6431961000004421557", BIG, amount="999999999.99", dbtr=BIG,
         reference="000008207791225857421286694", ustrd="€" * 140,
         alternative_schemes=["€" * 100, "€" * 100])
vec("QR-MAX-VERSION-25", "6.2", "Every accepted bill must encode within QR version 25 at ECC level M (else be rejected)",
    i, "accept_or_reject")

# --- Line definitions that must be rejected ----------------------------------
def reject(id, spec, description, input, known_gap=None):
    vec(id, spec, description, input, "reject", known_gap)


for amt, why in [("0050.00", "leading zeroes"), ("01.00", "leading zero")]:
    reject(f"AMT-REJECT-{amt}", "4.2.2 line 19", f"Amount with {why}", bill(BASE_IBAN, CDTR, amount=amt))
for amt, why in [("0.00", "zero amount without notification text"), ("1000000000.00", "amount above maximum"),
                 ("50,00", "comma decimal separator"), ("50", "no decimal places"), ("50.0", "one decimal place"),
                 ("50.000", "three decimal places"), ("-1.00", "negative amount"), ("1 590.00", "thousands separator")]:
    reject(f"AMT-REJECT-{amt}", "4.2.2 line 19", f"Amount: {why}", bill(BASE_IBAN, CDTR, amount=amt))

reject("CCY-USD", "4.2.2 line 20", "Only CHF and EUR", bill(BASE_IBAN, CDTR, currency="USD"))
reject("CCY-LOWERCASE", "4.2.2 line 20", "Currency code is case-sensitive ISO 4217", bill(BASE_IBAN, CDTR, currency="chf"))

reject("IBAN-DE", "4.2.2 line 4", "Only CH/LI IBANs", bill("DE89370400440532013000", CDTR))
reject("IBAN-CHECKSUM", "4.2.2 line 4", "IBAN with wrong check digits", bill("CH5900791123000889012", CDTR))
IBAN22 = iban_with_check("CH", "007911230008890123")
assert len(IBAN22) == 22 and mod97_ok(IBAN22)
reject("IBAN-LENGTH-22", "4.2.2 line 4", "CH IBAN with a valid checksum but 22 characters (fixed length 21)",
       bill(IBAN22, CDTR))
IBAN20 = iban_with_check("CH", "0079112300088901")
assert len(IBAN20) == 20 and mod97_ok(IBAN20)
reject("IBAN-LENGTH-20", "4.2.2 line 4", "CH IBAN with a valid checksum but 20 characters (fixed length 21)",
       bill(IBAN20, CDTR))

QR_IBAN = "CH4431999123000889012"
reject("QRIBAN-WITHOUT-QRR", "4.2.2 line 28 / 7.1", "QR-IBAN requires reference type QRR", bill(QR_IBAN, CDTR))
reject("QRIBAN-WITH-SCOR", "4.3.2", "A QR-IBAN cannot be used with SCOR", bill(QR_IBAN, CDTR, reference="RF18539007547034"))
reject("IBAN-WITH-QRR", "4.3.2", "A QR reference requires a QR-IBAN", bill(BASE_IBAN, CDTR, reference="210000000003139471430009017"))
reject("QRR-EUR", "2.10 / 2.12.1 / 4.3.2", "QR-IBAN and QR reference are for CHF only since v2.4",
       bill(QR_IBAN, CDTR, currency="EUR", amount="10.00", reference="210000000003139471430009017"),
       # Deferred until IG v2.3 expires in November 2027:
       # https://github.com/thomi137/swissqrust/issues/2
       known_gap="G7")

assert qrr_with_check("21000000000313947143000901") == "210000000003139471430009017"
reject("QRR-CHECK-DIGIT", "Annex B", "QR reference with wrong mod-10 check digit",
       bill(QR_IBAN, CDTR, reference="210000000003139471430009018"))
reject("QRR-ALL-ZEROS", "2.12.1", "QR reference must not consist exclusively of zeros",
       bill(QR_IBAN, CDTR, reference="0" * 27))
reject("QRR-26-DIGITS", "4.2.2 line 29", "QR reference is always 27 characters",
       bill(QR_IBAN, CDTR, reference="21000000000313947143000901"))
reject("SCOR-CHECK-DIGIT", "2.12.2", "Creditor Reference with wrong mod 97-10 check digits",
       bill(BASE_IBAN, CDTR, reference="RF19539007547034"))
reject("SCOR-26-CHARS", "2.12.2", "Creditor Reference longer than 25 characters",
       bill(BASE_IBAN, CDTR, reference="RF04GHJ74CV9B4DFH99RXPLMMQ4"))
reject("SCOR-NON-ALNUM", "4.1.2", "Creditor Reference is alphanumeric only",
       bill(BASE_IBAN, CDTR, reference="RF18-5390-0754-7034"))

reject("USTRD-141", "4.2.2 line 30", "Unstructured message above 140 characters", bill(BASE_IBAN, CDTR, ustrd="a" * 141))
reject("ADDINF-141-COMBINED", "4.3.3", "Ustrd + StrdBkgInf above 140 characters",
       bill(BASE_IBAN, CDTR, ustrd="a" * 100, bill_information="//S1/10/" + "1" * 33))
reject("ALTPMT-101", "4.2.2 lines 33-34", "Alternative procedure above 100 characters",
       bill(BASE_IBAN, CDTR, alternative_schemes=["a" * 101, None]))
reject("ALTPMT-3", "4.2.2 lines 33-34", "At most two alternative procedures",
       bill(BASE_IBAN, CDTR, alternative_schemes=["eBill/B/a@b.ch", "x/1", "y/2"]))
reject("CHARSET-EMOJI", "4.1.1", "Character outside the permitted set", bill(BASE_IBAN, CDTR, ustrd="Danke 🙂"))
reject("CHARSET-LF-IN-LINE", "4.1.1 / 4.1.4", "A line break inside a value would shift every following line",
       bill(BASE_IBAN, CDTR, ustrd="Zeile 1\nZeile 2"))
reject("CHARSET-CR-IN-NAME", "4.1.1 / 4.1.4", "Carriage return inside the name",
       bill(BASE_IBAN, addr("Muster\rAG", None, None, "8000", "Seldwyla", "CH")))
reject("CHARSET-TAB", "4.1.1", "Control characters are not permitted", bill(BASE_IBAN, CDTR, ustrd="a\tb"))

for field, a in [
    ("NAME-71", addr("a" * 71, None, None, "8000", "Seldwyla", "CH")),
    ("NAME-EMPTY", addr("", None, None, "8000", "Seldwyla", "CH")),
    ("STREET-71", addr("Muster AG", "a" * 71, None, "8000", "Seldwyla", "CH")),
    ("HOUSE-17", addr("Muster AG", "Musterstrasse", "1" * 17, "8000", "Seldwyla", "CH")),
    ("PLZ-17", addr("Muster AG", None, None, "1" * 17, "Seldwyla", "CH")),
    ("PLZ-EMPTY", addr("Muster AG", None, None, "", "Seldwyla", "CH")),
    ("TOWN-36", addr("Muster AG", None, None, "8000", "a" * 36, "CH")),
    ("TOWN-EMPTY", addr("Muster AG", None, None, "8000", "", "CH")),
    ("COUNTRY-UNKNOWN", addr("Muster AG", None, None, "8000", "Seldwyla", "XX")),
    ("COUNTRY-ALPHA3", addr("Muster AG", None, None, "8000", "Seldwyla", "CHE")),
]:
    reject(f"CDTR-{field}", "4.2.2 lines 5-11 / 4.3.1", f"Creditor address: {field.lower()}", bill(BASE_IBAN, a))
reject("DBTR-TOWN-EMPTY", "4.2.2 line 26", "Debtor town is dependent (mandatory when the debtor group is used)",
       bill(BASE_IBAN, CDTR, dbtr=addr("Sarah Beispiel", None, None, "8000", "", "CH")))

# --- Annex D billing information (SIX/Swico S1) ------------------------------
swico = [
    ("ANNEX-D-EX1", "//S1/10/10201409/11/190512/20/1400.000-53/30/106017086/31/180508/32/7.7/40/2:10;0:30",
     {"invoice_number": "10201409", "invoice_date": "2019-05-12", "customer_reference": "1400.000-53",
      "vat_number": "106017086", "vat_date": "2018-05-08", "vat_rate": 7.7,
      "payment_conditions": [{"discount": 2, "days": 10}, {"discount": 0, "days": 30}]}),
    ("ANNEX-D-EX2", "//S1/10/10104/11/180228/30/395856455/31/180226180227/32/3.7:400.19;7.7:553.39;0:14/40/0:30",
     {"invoice_number": "10104", "invoice_date": "2018-02-28", "vat_number": "395856455",
      "vat_start_date": "2018-02-26", "vat_end_date": "2018-02-27",
      "vat_rate_details": [{"rate": 3.7, "amount": 400.19}, {"rate": 7.7, "amount": 553.39}, {"rate": 0, "amount": 14}],
      "payment_conditions": [{"discount": 0, "days": 30}]}),
    ("ANNEX-D-EX3", "//S1/10/4031202511/11/180107/20/61257233.4/30/105493567/32/8:49.82/33/2.5:14.85/40/0:30",
     {"invoice_number": "4031202511", "invoice_date": "2018-01-07", "customer_reference": "61257233.4",
      "vat_number": "105493567", "vat_rate_details": [{"rate": 8, "amount": 49.82}],
      "vat_import_taxes": [{"rate": 2.5, "amount": 14.85}], "payment_conditions": [{"discount": 0, "days": 30}]}),
    ("ANNEX-D-EX4", "//S1/10/X.66711\\/8824/11/200712/20/MW-2020-04/30/107978798/32/2.5:117.22/40/3:5;1.5:20;1:40;0:60",
     {"invoice_number": "X.66711/8824", "invoice_date": "2020-07-12", "customer_reference": "MW-2020-04",
      "vat_number": "107978798", "vat_rate_details": [{"rate": 2.5, "amount": 117.22}],
      "payment_conditions": [{"discount": 3, "days": 5}, {"discount": 1.5, "days": 20},
                             {"discount": 1, "days": 40}, {"discount": 0, "days": 60}]}),
    ("ANNEX-A-2-S1", SWICO_A2, i2["swico_bill_information"]),
]

out = {
    "source": "SIX Interbank Clearing, Swiss Implementation Guidelines for the QR-bill, Version 2.4 (24.02.2026, "
              "valid from 14 November 2026): https://www.six-group.com/dam/download/banking-services/"
              "standardization/qr-bill/ig-qr-bill-v2.4-en.pdf",
    "notes": [
        "Expected payloads come from Table 8 and the Annex A examples (German original values, without the English "
        "translation glosses), not from this library.",
        "Lines are compared after dropping trailing empty status-A lines (StrdBkgInf, AltPmt): 4.1.4 says they are "
        "omitted when unused, while the Annex A tables show them as empty lines. Both forms are accepted.",
        "known_gap names a finding in tests/data/six/COMPLIANCE.md. Such vectors are expected to fail today; the runner "
        "fails once one passes, so the marker gets removed together with the fix.",
    ],
    "vectors": vectors,
    "billing_information": [
        {"id": id, "spec": f"{SPEC}, Annex D" if id.startswith("ANNEX-D") else f"{SPEC}, Annex A, Table 18",
         "swico_input": s, "text": t}
        for id, t, s in swico
    ],
}
path = sys.argv[1] if len(sys.argv) > 1 else "tests/data/six/ig_qr_bill_v2_4_vectors.json"
with open(path, "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=2)
    f.write("\n")
print(len(vectors), "vectors,", sum(1 for v in vectors if "known_gap" in v), "known gaps,", len(swico), "S1 vectors")
