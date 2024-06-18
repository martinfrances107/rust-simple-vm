use crate::parse::Parser;

use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Sequence<S, T, E, F> 
where F: Parser<S, T, E>
{
    _t: PhantomData<(S, T, E)>,
    items: Vec<F>,
}

impl<S, T, E, F> Sequence<S, T, E, F> 
where F: Parser<S, T, E>
{
    pub fn new(items: Vec<F>) -> Self {
        Self { items, _t: PhantomData::default() }
    }
}

impl<S, T, E, F: Parser<S, T, E>> Parser<S, Vec<T>, E> for Sequence<S, T, E, F> {
    fn run(&self, s: S) -> Result<(S, Vec<T>), E> {
        let mut state = s;
        let mut out = Vec::new();
        for i in &self.items {
            let (s, res) = i.run(state)?; 
            state = s;
            out.push(res);
        };
        Ok((state, out))
    }
}

#[derive(Debug, Default)]
pub struct Any<S, T, E, F>
where F: Parser<S, T, E>
{
    _t: PhantomData<(S, T, E)>,
    items: Vec<F>,
}

impl <S, T, E, F> Any<S, T, E, F> 
where F: Parser<S, T, E>
{
    pub fn new(items: Vec<F>) -> Self {
        Self {items, _t: PhantomData::default() }
    }
}

impl<S: Clone, T, E: Default, F: Parser<S, T, E>> Parser<S, T, E> for Any<S, T, E, F> {
    fn run(&self, s: S) -> Result<(S, T), E> {
        let mut last_err = E::default(); 
        for item in &self.items {
            match item.run(s.clone()) {
                Ok(x) => return Ok(x),
                Err(e) => last_err = e,
            };
        };
        Err(last_err)
    }
}

pub fn map<S, T, E, U, F, G>(f: F, g: G) -> impl Fn(S) -> Result<(S, U), E> 
where F: Parser<S,T,E>,
      G: Fn(T) -> U,
{
    move |input| {
        let (s, res) = f.run(input)?; 
        Ok((s, g(res)))
    }
}

pub fn require<S, T, E: Clone, F>(f: F, e: E) -> impl Fn(S) -> Result<(S, T), E>
where F: Parser<S, Option<T>, E>
{
    move |input| {
        let (s, res) = f.run(input)?;
        if let Some(t) = res {
            Ok((s, t))
        } else {
            Err(e.clone())
        }
    }
}

pub fn repeat0<S: Clone, T, E: Clone + Default, F>(f: F) -> impl Fn(S) -> Result<(S, Vec<T>), E> 
where F: Parser<S, T, E>
{
    move |input| {
        let mut out = Vec::new();
        let mut state = input;
        loop {
            if let Ok((s, res)) = f.run(state.clone()) {
                out.push(res);
                state = s;
            } else {
                return Ok((state, out))
            }
        }
    }
}

pub fn repeat1<S: Clone, T, E: Clone + Default, F>(f: F) -> impl Fn(S) -> Result<(S, Vec<T>), E> 
where F: Parser<S, T, E>
{
    move |input| {
        let mut out = Vec::new();
        let mut state = input;
        let last_err: E;
        loop {
            match f.run(state.clone()) {
                Ok((s, res)) => {
                    out.push(res);
                    state = s;
                }
                Err(e) => {
                    last_err = e;
                    break;
                }
            }
        };
        if out.len() < 1 {
            Err(last_err)
        } else {
            Ok((state, out))
        }
    }
}

pub fn discard<S, T, E, F>(f: F) -> impl Fn(S) -> Result<(S, ()), E> 
where F: Parser<S, T, E>
{
    map(f, |_| ())
}

pub fn wrapped<A, B, C, S, E, F, G, H>(f: F, g: G, h: H) -> impl Fn(S) -> Result<(S, B), E> 
where F: Parser<S, A, E>,
      G: Parser<S, B, E>,
      H: Parser<S, C, E>,
{
    move |input| {
        let (input0, _) = f.run(input)?;
        let (input1, res) = g.run(input0)?;
        let (input2, _) = h.run(input1)?;
        Ok((input2, res))
    }
}

pub fn delimited<A, B, S: Clone, E, F, G>(f: F, delim: G) -> impl Fn(S) -> Result<(S, Vec<A>), E>
where F: Parser<S, A, E>,
      G: Parser<S, B, E>,
{
    move |input| {
        let mut out = Vec::new();
        let mut current_state = input;
        loop {
            let (sn, res) = f.run(current_state)?; 
            out.push(res);
            match delim.run(sn.clone()) {
                Ok((snn, _)) => current_state = snn,
                Err(_) => return Ok((sn, out)),
            };
        }
    }
}

pub fn allow_empty<S: Clone, T, E, F>(f: F) -> impl Fn(S) -> Result<(S, Vec<T>), E> 
where F: Parser<S, Vec<T>, E>,
{
    move |input| {
        match f.run(input.clone()) {
            Ok(x) => Ok(x),
            Err(_) => Ok((input, Vec::new())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::character::*;

    #[test]
    fn test_wrapped() {
        {
            let (s, n) = wrapped(token("{"), alpha, token("}"))("{a}").unwrap();
            assert_eq!('a', n); 
            assert_eq!("", s); 
        }
        {
            let (s, n) = wrapped(token("<<"), map(repeat1(alpha), |x| x.iter().collect::<String>()), token("}}"))("<<foobar}}").unwrap();
            assert_eq!("foobar", n); 
            assert_eq!("", s); 
        }
        {
            let (s, n) = wrapped(
                token("\""), 
                map(repeat1(not_char("\"")), |x| x.iter().collect::<String>()),
                token("\""),
            )("\"
Hello world this is some text. \"").unwrap();
            assert_eq!("\nHello world this is some text. ", n);
            assert_eq!("", s);
        }
    }

    #[test]
    fn test_any() {
        let (s, n) = Any::new(vec![alpha, numeric]).run("1").unwrap();
        assert_eq!("", s);
        assert_eq!('1', n);
    }

    #[test]
    fn test_delimited() {
        let (s, n) = delimited(alpha, token(","))("a,b,c,d").unwrap();
        assert_eq!("", s);
        assert_eq!(vec!['a', 'b', 'c', 'd'], n);
    }
}
