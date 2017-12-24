use super::doc::*;
use std::collections::HashMap;

fn place_chars(res: &mut DocSpan, value: String) {
    if !res.is_empty() {
        let idx = res.len() - 1;
        if let DocChars(ref mut prefix) = res[idx] {
            prefix.push_str(&value[..]);
            return;
        }
    }
    res.push(DocChars(value));
}

fn place_any(res: &mut DocSpan, value: &DocElement) {
    match *value {
        DocChars(ref string) => {
            place_chars(res, string.clone());
        }
        _ => {
            res.push(value.clone());
        }
    }
}

fn place_many(res: &mut DocSpan, values: &[DocElement]) {
    if !values.is_empty() {
        place_any(res, &values[0]);
        res.extend_from_slice(&values[1..]);
    }
}

fn apply_add_inner(spanvec: &DocSpan, delvec: &AddSpan) -> (DocSpan, DocSpan) {
    let mut span = &spanvec[..];
    let mut del = &delvec[..];

    let mut first = None;
    if !span.is_empty() {
        first = Some(span[0].clone());
        span = &span[1..]
    }

    let mut res: DocSpan = Vec::with_capacity(span.len());

    if del.is_empty() {
        return (vec![], spanvec.clone().to_vec());
    }

    let mut d = del[0].clone();
    del = &del[1..];

    let mut exhausted = first.is_none();

    trace!("ABOUT TO APPLY ADD {:?} {:?}", first, span);

    loop {
        // Flags for whether we have partially or fully consumed an atom.
        let mut nextdel = true;
        let mut nextfirst = true;

        if exhausted {
            match d {
                AddSkip(..) | AddWithGroup(..) => {
                    panic!("exhausted document on {:?}", d);
                }
                _ => {}
            }
        }

        trace!("next {:?} {:?} {:?}", d, first, exhausted);

        match d.clone() {
            AddSkip(count) => {
                match first.clone().unwrap() {
                    DocChars(ref value) => {
                        let len = value.chars().count();
                        if len < count {
                            place_chars(&mut res, value.to_owned());
                            d = AddSkip(count - len);
                            nextdel = false;
                        } else if len > count {
                            place_chars(&mut res, value[0..count].to_owned());
                            first = Some(DocChars(value[count..len].to_owned()));
                            nextfirst = false;
                        } else {
                            place_chars(&mut res, value.to_owned());
                        }
                    }
                    DocGroup(..) => {
                        res.push(first.clone().unwrap());
                        if count > 1 {
                            d = AddSkip(count - 1);
                            nextdel = false;
                        }
                    }
                }
            }
            AddWithGroup(ref delspan) => {
                match first.clone().unwrap() {
                    DocGroup(ref attrs, ref span) => {
                        res.push(DocGroup(attrs.clone(), apply_add(span, delspan)));
                    }
                    _ => {
                        panic!("Invalid AddWithGroup");
                    }
                }
            }
            AddChars(value) => {
                place_chars(&mut res, value);
                nextfirst = false;
            }
            AddGroup(attrs, innerspan) => {
                let mut subdoc = vec![];
                if !exhausted {
                    subdoc.push(first.clone().unwrap());
                    subdoc.extend_from_slice(span);
                }
                trace!("CALLING INNER {:?} {:?}", subdoc, innerspan);

                let (inner, rest) = apply_add_inner(&subdoc, &innerspan);
                place_any(&mut res, &DocGroup(attrs, inner));

                trace!("REST OF INNER {:?} {:?}", rest, del);

                let inner = apply_add(&rest, &del.to_vec());
                place_many(&mut res, &inner);
                return (res, vec![]);
            }
        }

        if nextdel {
            if del.is_empty() {
                let mut remaining = vec![];
                if !nextfirst && !first.is_none() && !exhausted {
                    remaining.push(first.clone().unwrap());
                    // place_any(&mut res, &first.clone().unwrap());
                }
                remaining.extend_from_slice(span);
                return (res, remaining);
            }

            d = del[0].clone();
            del = &del[1..];
        }

        if nextfirst {
            if span.is_empty() {
                exhausted = true;
            } else {
                first = Some(span[0].clone());
                span = &span[1..];
            }
        }
    }
}

pub fn apply_add(spanvec: &DocSpan, delvec: &AddSpan) -> DocSpan {
    let (mut res, remaining) = apply_add_inner(spanvec, delvec);

    // TODO never accept unbalanced components?
    if !remaining.is_empty() {
        place_many(&mut res, &remaining);
        // panic!("Unbalanced apply_add");
    }
    res
}

pub fn apply_delete(spanvec: &DocSpan, delvec: &DelSpan) -> DocSpan {
    let mut span = &spanvec[..];
    let mut del = &delvec[..];

    let mut res: DocSpan = Vec::with_capacity(span.len());

    if del.is_empty() {
        return span.to_vec();
    }

    let mut first = span[0].clone();
    span = &span[1..];

    let mut d = del[0].clone();
    del = &del[1..];

    loop {
        let mut nextdel = true;
        let mut nextfirst = true;

        match d.clone() {
            DelObject => {
                panic!("TODO");
            }
            DelSkip(count) => {
                match first.clone() {
                    DocChars(ref value) => {
                        let len = value.chars().count();
                        if len < count {
                            place_chars(&mut res, value.clone());
                            d = DelSkip(count - len);
                            nextdel = false;
                        } else if len > count {
                            place_chars(&mut res, value[0..count].to_owned());
                            first = DocChars(value[count..len].to_owned());
                            nextfirst = false;
                        } else {
                            place_chars(&mut res, value.clone());
                            nextdel = true;
                        }
                    }
                    DocGroup(..) => {
                        res.push(first.clone());
                        if count > 1 {
                            d = DelSkip(count - 1);
                            nextdel = false;
                        }
                    }
                }
            }
            DelWithGroup(ref delspan) => {
                match first.clone() {
                    DocGroup(ref attrs, ref span) => {
                        res.push(DocGroup(attrs.clone(), apply_delete(span, delspan)));
                    }
                    _ => {
                        panic!("Invalid DelWithGroup");
                    }
                }
            }
            DelGroup(ref delspan) => {
                match first.clone() {
                    DocGroup(ref attrs, ref span) => {
                        place_many(&mut res, &apply_delete(span, delspan)[..]);
                    }
                    _ => {
                        panic!("Invalid DelGroup");
                    }
                }
            }
            DelChars(count) => {
                match first.clone() {
                    DocChars(ref value) => {
                        let len = value.chars().count();
                        if len > count {
                            first = DocChars(value[count..].to_owned());
                            nextfirst = false;
                        } else if len < count {
                            panic!("attempted deletion of too much");
                        }
                    }
                    _ => {
                        panic!("Invalid DelChars");
                    }
                }
            }
            DelGroupAll => {
                match first.clone() {
                    DocGroup(..) => {}
                    _ => {
                        panic!("Invalid DelGroupAll");
                    }
                }
            }
        }

        if nextdel {
            if del.is_empty() {
                if !nextfirst {
                    place_any(&mut res, &first)
                }
                if !span.is_empty() {
                    place_any(&mut res, &span[0]);
                    res.extend_from_slice(&span[1..]);
                }
                break;
            }

            d = del[0].clone();
            del = &del[1..];
        }

        if nextfirst {
            if span.is_empty() {
                panic!("exhausted document\n -->{:?}\n -->{:?}", first, span);
            }

            first = span[0].clone();
            span = &span[1..];
        }
    }

    res
}

pub fn apply_operation(spanvec: &DocSpan, op: &Op) -> DocSpan {
    let &(ref delvec, ref addvec) = op;
    let postdel = apply_delete(spanvec, delvec);
    apply_add(&postdel, addvec)
}

fn normalize_del(mut del: DelSpan) -> DelSpan {
    // let mut tail = true;
    // del.into_iter().rev().map(|x| {
    //     //TODO
    //     x
    // }).filter(move |x| {
    //     match x {
    //         &DelSkip(_) => {
    //             false
    //         },
    //         _ => true
    //     }
    // }).rev().collect()
    if let Some(&DelSkip(..)) = del.last() {
        del.pop();
    }
    del
}

pub fn normalize(op: Op) -> Op {
    // TODO all
    (normalize_del(op.0), op.1)
}