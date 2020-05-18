// LNP/BP Core Library implementing LNPBP specifications & standards
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use super::{Error, Read, Write};
use core::borrow::Borrow;

impl Read for zmq::Socket {
    fn read(&mut self) -> Result<Vec<u8>, Error> {
        Ok(self.recv_bytes(0)?)
    }
}

impl Write for zmq::Socket {
    fn write(&mut self, data: impl Borrow<[u8]>) -> Result<usize, Error> {
        self.send(data.borrow(), 0)?;
        Ok(data.borrow().len())
    }
}