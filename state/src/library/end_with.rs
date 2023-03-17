use rxrust::{prelude::*, rc::{MutRc, RcDeref, RcDerefMut}};

#[derive(Clone)]
pub struct EndWithOp<S, B> {
  pub(crate) source: S,
  pub(crate) values: Vec<B>,
}

impl<Item, Err, O, S> Observable<Item, Err, O> for EndWithOp<S, Item>
where
  S: Observable<Item, Err, EndWithObserver<MutRc<Option<O>>, Item>>,
  O: Observer<Item, Err>,
{
  type Unsub = S::Unsub;

  fn actual_subscribe(self, observer: O) -> Self::Unsub {
    self.source.actual_subscribe(EndWithObserver {
        next: MutRc::own(Some(observer)),
        values: self.values,
    })
  }
}

pub struct EndWithObserver<O, B>
{
    pub(crate) next: O,
    pub(crate) values: Vec<B>,
}

impl<Item, Err, O> Observer<Item, Err> for EndWithObserver<MutRc<Option<O>>, Item>
where
    O: Observer<Item, Err>,
{
    fn next(&mut self, value: Item) {
        if let Some(data) = self.next.rc_deref_mut().as_mut() {
            data.next(value)
        }
    }
    
    fn error(self, err: Err) {
        if let Some(data) = self.next.rc_deref_mut().take() {
            data.error(err)
        }
    }

    fn complete(self) {
        if let Some(mut data) = self.next.rc_deref_mut().take() {
            for val in self.values {
                data.next(val);
            }
            data.complete();
        }
    }

    fn is_finished(&self) -> bool {
        self.next.rc_deref().as_ref().map_or(true, |data| {
            data.is_finished()
        })
    }
}

impl<Item, Err, S> ObservableExt<Item, Err> for EndWithOp<S, Item>
where
  S: ObservableExt<Item, Err>
{
}

pub trait EndWithExt<Item, Err>: ObservableExt<Item, Err>
where
    Self: Sized,
{
    fn end_with<B>(self, values: Vec<B>) -> EndWithOp<Self, B> {
        EndWithOp { source: self, values }
    }
}

impl<Item, Err, S> EndWithExt<Item, Err> for S
where
  S: ObservableExt<Item, Err>
{
}

#[cfg(test)]
mod test {
  use rxrust::{prelude::*, of_sequence};
  use super::EndWithExt;

  #[test]
  fn simple_integer() {
    let mut ret = String::new();

    {
      let s = of_sequence!(1, 2, 3);

      s.end_with(vec![4, 5]).subscribe(|value| {
        ret.push_str(&value.to_string());
      });
    }

    assert_eq!(ret, "12345");
  }
}
