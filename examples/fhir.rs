//! HL7 FHIR Quantity interoperability example with fallbacks and conversion.
//!
//! Run with: `cargo run --example fhir`

use ucum::{FhirQuantity, Quantity};

fn main() {
    // 1. Standard conversion from a Quantity to FHIR
    let q = Quantity::new(125.0, "mg/dL");
    let fhir = q.to_fhir();

    println!("--- Standard FHIR Quantity ---");
    println!("  value:  {:?}", fhir.value);
    println!("  system: {:?}", fhir.system);
    println!("  code:   {:?}", fhir.code);

    let roundtrip = Quantity::try_from_fhir(&fhir).unwrap();
    println!("  Round-trip: {} {}", roundtrip.value, roundtrip.unit);
    println!();

    // 2. Handling real-world FHIR payloads (e.g., using 'unit' fallback and no system URI)
    let loose_fhir = FhirQuantity {
        value: Some(5.0),
        system: None,
        code: None,
        unit: Some("mmol/L".to_string()),
    };

    println!("--- Loose FHIR Payload (Fallback & Conversion) ---");
    let parsed_q = Quantity::try_from_fhir(&loose_fhir).unwrap();
    println!("  Parsed unit: {}", parsed_q.unit);

    // Perform conversion on commensurable molar concentration units
    let converted = parsed_q.convert_to("umol/L").unwrap();
    println!("  Converted value: {} {}", converted.value, converted.unit);
}
