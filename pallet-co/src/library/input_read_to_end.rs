// use std::io::ErrorKind;

/*
trait ReadToEnd {
    fn read_to_end(input: &mut Self) -> Result<Vec<u8>, codec::Error>;
}

impl ReadToEnd for codec::Input {
    fn read_to_end(input: &mut T) -> Result<Vec<u8>, codec::Error> {
        
    }
}
*/

pub fn input_read_to_end<I: codec::Input>(input: &mut I) -> Result<Vec<u8>, codec::Error> {
    // read on one go when have remanining length
    match input.remaining_len() {
        Ok(Some(len)) => {
            let mut data = Vec::<u8>::with_capacity(len);
            input.read(&mut data[..])?;
            return Ok(data);
        },
        _ => {},
    }

    // read byte by byte
    let mut data = Vec::<u8>::new();
    loop {
        match input.read_byte() {
            Ok(c) => data.push(c),
            Err(e) => break, // TODO: Only break on EOF.
            // Err(e) => e.as_ref(),
            // Err(e) => {
            //     // e.downcast_ref::<std::io::Error>()
            //     match std::error::downcast_ref::<std::io::Error>(e) {
            //         Ok(io) => match io.kind() {
            //             ErrorKind::Interrupted => continue,
            //             ErrorKind::UnexpectedEof => break,
            //             _ => {}
            //         }
            //         None => {}
            //     }
            //     return Err(e);
            // },
        }
    }
    Ok(data)
}
