// Configurazione foglio A4 e margini
#set page(
  paper: "a4",
  margin: (x: 1.8cm, top: 1.5cm, bottom: 1.5cm),
)
#set text(
  font: ("Arial", "DejaVu Sans", "Liberation Sans"),
  size: 9.5pt,
  lang: "it",
)

// Intestazione
#align(center)[
  #text(size: 13pt, weight: "bold")[ASSOCIAZIONE SPORTIVA DILETTANTISTICA SHINE] \
  #text(size: 10.5pt, fill: rgb("#333"))[Domanda di Iscrizione e Tesseramento Stagione 2026/2027]
]

#v(1mm)
#line(length: 100%, stroke: 1.5pt + rgb("#000"))
#v(2mm)

// Helper grafico per i campi
#let field(label, value) = [
  #text(fill: rgb("#4b5563"))[#label: ] *#value*
]

// 1. Dati Anagrafici
#rect(fill: rgb("#f3f4f6"), width: 100%, inset: (x: 6pt, y: 5pt), radius: 2pt)[
  *1. Dati Anagrafici Richiedente (Maggiorenne o Genitore / Tutore)*
]
#v(1mm)

#grid(
  columns: (1fr, 1fr),
  row-gutter: 7pt,
  field("Cognome", "{{ last_name }}"), field("Nome", "{{ first_name }}"),
  field("Nato/a a", "{{ birth_place }} ({{ birth_province }})"), field("Data di nascita", "{{ birth_date }}"),
  field("Codice Fiscale", "{{ fiscal_code }}"), field("Telefono", "{{ phone }}"),
)
#v(3pt)
#field(
  "Indirizzo di Residenza",
  "{{ residence_address }} n. {{ residence_number }}, {{ residence_cap }} {{ residence_city }} ({{ residence_province }})",
) \
#v(3pt)
#field("E-mail", "{{ email }}")

#v(3mm)

// 2. Sezione Minore (Opzionale)
{% if is_minor.is_some() %}
#rect(fill: rgb("#f3f4f6"), width: 100%, inset: (x: 6pt, y: 5pt), radius: 2pt)[
  *2. Dati del Tesserato Minorenne*
]
#v(1mm)
#grid(
  columns: (1fr, 1fr),
  row-gutter: 7pt,
  field("Cognome Minore", "{{ minor_last_name.as_deref().unwrap_or("") }}"), field("Nome Minore", "{{ minor_first_name.as_deref().unwrap_or("") }}"),
  field("Nato/a a", "{{ minor_birth_place.as_deref().unwrap_or("") }} ({{ minor_birth_province.as_deref().unwrap_or("") }})"),
  field("Data di nascita", "{{ minor_birth_date.as_deref().unwrap_or("") }}"),

  field("Codice Fiscale", "{{ minor_fiscal_code.as_deref().unwrap_or("") }}"),
  field("Residenza", "{{ minor_residence_city.as_deref().unwrap_or("") }} ({{ minor_residence_province.as_deref().unwrap_or("") }})"),
)
#v(3mm)
{% endif %}

// 3. Consensi Privacy
#rect(fill: rgb("#f3f4f6"), width: 100%, inset: (x: 6pt, y: 5pt), radius: 2pt)[
  *3. Consensi e Trattamento Dati*
]
#v(2mm)

- Consenso all'utilizzo di foto e riprese video istituzionali: \
  *{% if consent_photo %} [X] ACCONSENTO {% else %} [ ] NON ACCONSENTO {% endif %}*

#v(1mm)
- Consenso alla pubblicazione sui canali social e media dell'Associazione: \
  *{% if consent_publication %} [X] ACCONSENTO {% else %} [ ] NON ACCONSENTO {% endif %}*

#v(8mm)

// 4. Luogo, Data e Firma Digitale
#grid(
  columns: (1fr, 1fr),
  align: (left + bottom, center + bottom),
  [
    #field("Luogo e Data", "{{ place_and_date }}")
    #v(6mm)
  ],
  [
    #text(size: 8.5pt, fill: rgb("#555"))[Firma del richiedente / tutore legale:] \
    #v(1mm)
    // Typst caricherà i byte della firma dalla memoria virtuale di Rust
    #image("signature.png", height: 26mm)
    #line(length: 60mm, stroke: 0.5pt + rgb("#666"))
  ],
)
