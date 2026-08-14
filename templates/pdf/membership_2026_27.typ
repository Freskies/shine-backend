// Page setup and document style
#set page(
  paper: "a4",
  margin: (x: 1.5cm, top: 1.0cm, bottom: 1.0cm),
)

#set text(
  font: "Liberation Sans",
  size: 9pt,
  lang: "it",
)

#set par(justify: true, leading: 0.52em)

// ----------------------------------------------------
// HELPER FUNCTIONS
// ----------------------------------------------------

// Highlight function for dynamic/form data (con fallback a puntini se vuoto)
#let form_data(body, dots: none) = {
  // Verifica se il contenuto passato è vuoto o composto solo da spazi
  let is_empty = (
    body == []
    or body == none
    or (type(body) == str and body.trim() == "")
    or (type(body) == content and body.has("text") and body.text.trim() == "")
  )

  if is_empty {
    if dots != none {
      text(fill: rgb("#777"))[#dots]
    } else {
      none
    }
  } else {
    text(weight: "bold", fill: rgb("#0b4f8a"))[#highlight(fill: rgb("#e8f0fe"), radius: 2pt, top-edge: "ascender", bottom-edge: "descender")[#body]]
  }
}

// Checkbox helper
#let check_box(checked: false) = {
  box(
    width: 9pt,
    height: 9pt,
    stroke: 0.8pt + black,
    baseline: 1pt,
    inset: 1pt,
    radius: 1pt,
    if checked { text(size: 7pt, weight: "bold")[✓] } else { none }
  )
}

// Signature and date section helper designed for Rust virtual memory injection
#let signature_row(
  place_and_date: "Ravenna, 15/09/2026",
  signature_image: none, // e.g., image("signature.png", height: 16mm)
  label: "Firma",
) = {
  grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    align: (left + bottom, center + bottom),
    [
      Luogo e data #form_data(place_and_date)
      #v(2mm)
    ],
    [
      #if signature_image != none [
        #align(center)[
          #signature_image
          #v(-2mm)
        ]
      ] else [
        #v(14mm) // Spacing placeholder when no signature image is injected
      ]
      #line(length: 100%, stroke: 0.5pt + rgb("#333"))
      #v(-2pt)
      #text(size: 7.5pt, fill: rgb("#555"))[#label]
    ]
  )
}

// ----------------------------------------------------
// HEADER: LOGOS & TITLE
// ----------------------------------------------------
#grid(
  columns: (70pt, 1fr, 75pt),
  gutter: 10pt,
  align: (center + horizon, center + horizon, center + horizon),
  [
    #image("logo_shine.png", width: 100%)
  ],
  [
    #set text(size: 8.5pt)
    *ASSOCIAZIONE SPORTIVA DILETTANTISTICA SHINE* \
    Via Piceno 2 \
    48121, Ravenna \
    #v(2pt)
    #text(size: 11pt, weight: "bold")[Domanda di tesseramento per l'anno] \
    #text(size: 12.5pt, weight: "bold")[2026/2027]
  ],
  [
    #image("logo_uisp.png", width: 100%)
  ]
)

#v(2pt)

// ----------------------------------------------------
// SECTION 1: ADULT / PARENT OR LEGAL GUARDIAN
// ----------------------------------------------------
#block(
  fill: rgb("#f5f5f7"),
  inset: (x: 6pt, y: 3.5pt),
  radius: 3pt,
  width: 100%,
  [*La parte sottostante è da compilare se si è maggiorenne o in qualità di genitore e/o legale rappresentante di un minore:*]
)

#v(1pt)

Il/la sottoscritto/a (cognome) #form_data[{{ last_name }}] (nome) #form_data[{{ first_name }}]
nato/a a #form_data[{{ birth_place }}] (#form_data[{{ birth_province }}]) il #form_data[{{ birth_date }}], residente a #form_data[{{ residence_city }}]
in Via/Piazza #form_data[{{ residence_address }}] n° #form_data[{{ residence_number }}], CAP #form_data[{{ residence_cap }}], Prov. #form_data[{{ residence_province }}]
cellulare #form_data[{{ phone }}], e-mail (obbligatoria)\* #form_data[{{ email }}]
#text(size: 8pt)[*\*la tessera UISP sarà spedita via e-mail*], Codice Fiscale #form_data[{{ fiscal_code }}]

#v(4pt)

// ----------------------------------------------------
// SECTION 2: MINOR DETAILS (con puntini di fallback)
// ----------------------------------------------------
#text(weight: "bold")[NB: la parte sottostante è da compilare solo in caso di tesseramento di un minore:]
In qualità di genitore e/o legale rappresentante del minore:
(cognome del minore) #form_data(dots: "........................................")[{{ minor_last_name.as_deref().unwrap_or("") }}] (nome del minore) #form_data(dots: "........................................")[{{ minor_first_name.as_deref().unwrap_or("") }}]
nato a #form_data(dots: "........................................")[{{ minor_birth_place.as_deref().unwrap_or("") }}] (#form_data(dots: "..........")[{{ minor_birth_province.as_deref().unwrap_or("") }}]), il #form_data(dots: "....................")[{{ minor_birth_date.as_deref().unwrap_or("") }}], residente a #form_data(dots: "........................................")[{{ minor_residence_city.as_deref().unwrap_or("") }}]
in Via/Piazza #form_data(dots: "........................................")[{{ minor_residence_address.as_deref().unwrap_or("") }}] n° #form_data(dots: "..........")[{{ minor_residence_number.as_deref().unwrap_or("") }}] CAP #form_data(dots: "..........")[{{ minor_residence_cap.as_deref().unwrap_or("") }}] Prov. #form_data(dots: "..........")[{{ minor_residence_province.as_deref().unwrap_or("") }}]
Codice Fiscale #form_data(dots: "........................................")[{{ minor_fiscal_code.as_deref().unwrap_or("") }}]

#v(3pt)

// ----------------------------------------------------
// SECTION 3: REQUEST & DECLARATION
// ----------------------------------------------------
#align(center)[#text(weight: "bold", size: 9.5pt)[CHIEDE]]
#v(-4pt)
di poter essere ammesso/di ammettere il minore, in qualità di Atleta all'"ASSOCIAZIONE SPORTIVA DILETTANTISTICA SHINE". Inoltre, il/la sottoscritto/a

#v(1pt)
#align(center)[#text(weight: "bold", size: 9.5pt)[DICHIARA]]
#v(-4pt)

#set list(indent: 0pt, body-indent: 4pt, spacing: 4pt)

- di aver preso visione dello Statuto e dei Regolamenti dell'Associazione e di accettarli e rispettarli in ogni loro punto (#link("https://www.shineparkour.com/statute")[https://www.shineparkour.com/statute]);
- d'impegnarsi al pagamento della Quota Associativa annuale;
- di essere consapevole che le lezioni potranno svolgersi anche all'esterno, in presenza degli istruttori; • di acconsentire al trattamento dei dati personali da parte dell'Associazione, ai sensi dell'art.13 del Regolamento UE/2016/679 e ai sensi dell'art. 13 del DLgs 30/06/2003 n. 196 e in relazione all'informativa fornita. \
  In particolare si presta il consenso al trattamento dei dati personali per la realizzazione delle finalità istituzionali dell'Associazione, nella misura necessaria all'adempimento di obblighi previsti dalla legge e dalle norme statutarie. Firmando dichiara di aver preso visione dei moduli "informativa sulla privacy" disponibili a questi link \
  #link("https://www.shineparkour.com/privacy_policy")[https://www.shineparkour.com/privacy_policy] \
  #link("https://drive.google.com/open?id=1Em2gAzZxTiG8DOuoimP9kzN664DKtgft")[https://drive.google.com/open?id=1Em2gAzZxTiG8DOuoimP9kzN664DKtgft] e di accettarli.
- si autorizza la fotografia e/o la ripresa del sottoscritto / del minore, effettuate ai soli fini istituzionali, durante lo svolgimento delle attività e/o delle manifestazioni organizzate dall'Associazione.
  #align(center)[
    Sì #check_box(checked: {{ consent_photo }}) #h(35pt) No #check_box(checked: {{ !consent_photo }})
  ]
- Si acconsente al trattamento e alla pubblicazione, per i soli fini istituzionali, di video, fotografie e/o immagini atte a rivelare l'identità del sottoscritto/del minore, sul web e su tutti i mezzi di comunicazione utilizzati da SHINE ASD.
  #align(center)[
    Sì #check_box(checked: {{ consent_publication }}) #h(35pt) No #check_box(checked: {{ !consent_publication }})
  ]

#v(2pt)

// First signature block (General membership / Privacy consent)
#signature_row(
  place_and_date: "{{ place_and_date }}",
  signature_image: image("signature.png", height: 16mm), // Rust virtual file / asset
  label: "Firma del richiedente / tutore legale",
)

#v(4pt)

// ----------------------------------------------------
// SECTION 4: AUTONOMY (COMMUTE)
// ----------------------------------------------------
#text(weight: "bold")[NB: la parte sottostante è da compilare solo in caso in cui il minore compia in autonomia i tragitti di andata e ritorno]

#v(1pt)

Il sottoscritto, in qualità di genitore e/o di legale rappresentante del minore, si assume la piena responsabilità degli spostamenti 'casa-luogo di incontro per gli allenamenti' e 'luogo di incontro per gli allenamenti-casa' che il minore svolge in autonomia o comunque non accompagnato da legale rappresentante, liberando SHINE a.s.d. da ogni responsabilità. In fede,

#v(2pt)

// Second signature block (Commute autonomy release)
#signature_row(
  place_and_date: "{{ autonomy_place_and_date.as_deref().unwrap_or("") }}",
  {% if autonomy_signature.is_some() %}signature_image: image("signature2.png", height: 16mm), // Rust virtual file / asset{% else %}signature_image: none,{% endif %}
  label: "Firma del genitore / tutore legale",
)