use serde::Serialize;

pub fn to_csv<T: Serialize>(records: &[T]) -> Result<String, csv::Error> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    for item in records {
        wtr.serialize(item)?;
    }
    let data = wtr.into_inner().map_err(|e| e.into_error())?;
    String::from_utf8(data).map_err(|_| {
        csv::Error::from(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "CSV output was not valid UTF-8",
        ))
    })
}
