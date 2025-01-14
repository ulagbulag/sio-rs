// Copyright (c) 2019 Andreas Auernhammer. All rights reserved.
// Use of this source code is governed by a license that can be
// found in the LICENSE file.

use crate::error::{Invalid, NotAuthentic};
use std::marker::PhantomData;

pub trait Algorithm {
    const KEY_LEN: usize;
    const NONCE_LEN: usize;
    const TAG_LEN: usize;

    fn new(key: &[u8; 32], nonce: Nonce) -> Self;

    fn seal_in_place<'a>(
        &mut self,
        aad: &[u8],
        in_out: &'a mut Vec<u8>,
    ) -> Result<&'a [u8], Invalid>;

    fn open_in_place<'a>(
        &mut self,
        aad: &[u8],
        in_out: &'a mut [u8],
    ) -> Result<&'a [u8], NotAuthentic>;
}

pub struct Key<A: Algorithm>([u8; 32], PhantomData<A>);

impl<A: Algorithm> Key<A> {
    pub const SIZE: usize = A::KEY_LEN;

    pub fn new(bytes: [u8; 32]) -> Self {
        Key(bytes, PhantomData)
    }
}

impl<A: Algorithm> AsRef<[u8; 32]> for Key<A> {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Copy, Clone)]
pub struct Nonce([u8; 8]);

impl Nonce {
    pub const SIZE: usize = ::ring::aead::NONCE_LEN - 4;

    pub fn new(bytes: [u8; 8]) -> Self {
        Nonce(bytes)
    }
}

impl AsRef<[u8; 8]> for Nonce {
    fn as_ref(&self) -> &[u8; 8] {
        &self.0
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Aad<'a, A: Algorithm>(&'a [u8], PhantomData<A>);

impl<A: Algorithm> Aad<'static, A> {
    #[inline]
    pub fn empty() -> Self {
        Aad(&[], PhantomData)
    }
}

impl<'a, A: Algorithm> Copy for Aad<'a, A> {}

impl<'a, A: Algorithm> Clone for Aad<'a, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, A: Algorithm> AsRef<[u8]> for Aad<'a, A> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

impl<'a, A: Algorithm> From<&'a [u8]> for Aad<'a, A> {
    #[inline]
    fn from(aad: &'a [u8]) -> Self {
        Aad(aad, PhantomData)
    }
}

pub(crate) struct Counter {
    nonce: [u8; 12],
    pub seq_num: u32,
    exceeded: bool,
}

impl Counter {
    fn new(nonce: Nonce, seq_num: u32) -> Self {
        let mut value = [0; 12];
        value[..8].copy_from_slice(&nonce.0);
        Counter {
            nonce: value,
            seq_num,
            exceeded: false,
        }
    }

    #[inline]
    pub fn zero(nonce: Nonce) -> Self {
        Self::new(nonce, 0)
    }

    #[inline]
    pub fn one(nonce: Nonce) -> Self {
        Self::new(nonce, 1)
    }
}

impl ::ring::aead::NonceSequence for Counter {
    #[inline]
    fn advance(&mut self) -> Result<::ring::aead::Nonce, ::ring::error::Unspecified> {
        if self.exceeded {
            return Err(::ring::error::Unspecified);
        }

        self.nonce[8..].copy_from_slice(self.seq_num.to_le_bytes().as_ref());
        if let Some(seq_num) = self.seq_num.checked_add(1) {
            self.seq_num = seq_num;
        } else {
            self.exceeded = true;
        }
        Ok(::ring::aead::Nonce::assume_unique_for_key(self.nonce))
    }
}
