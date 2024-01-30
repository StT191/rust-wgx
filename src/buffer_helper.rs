
use std::{
  mem::size_of, ptr::copy_nonoverlapping,
  slice::SliceIndex, ops::{Range, RangeBounds, Bound}, cmp::Ordering, marker::PhantomData,
};
use crate::{*, error::*};

// data buffer

#[derive(Debug)]
pub struct DataBuffer<T: Copy + ReadBytes, D: AsRef<[T]>> {
  pub vertex_type: PhantomData<T>,
  pub data: D,
  pub buffer: wgpu::Buffer,
}

impl<T: Copy + ReadBytes, D: AsRef<[T]>> DataBuffer<T, D> {

  pub fn size(&self) -> u64 { self.buffer.size() }
  pub fn len(&self) -> usize { self.data.as_ref().len() }
  pub fn is_empty(&self) -> bool { self.data.as_ref().is_empty() }

  pub fn new(gx: &impl WgxDevice, usage: BufUse, size: usize, data: D) -> Self {
    Self {
      vertex_type: PhantomData, data,
      buffer: gx.buffer(BufUse::COPY_DST | usage, (size_of::<T>() * size) as u64, false),
    }
  }

  pub fn from_data(gx: &impl WgxDevice, usage: BufUse, data: D) -> Self {
    Self {
      buffer: gx.buffer_from_data(BufUse::COPY_DST | usage, data.as_ref()),
      vertex_type: PhantomData, data,
    }
  }

  pub fn write_buffer(&self, gx: &impl WgxDeviceQueue, range: impl SliceIndex<[T], Output = [T]> + RangeBounds<usize> + Clone) {

    let data_slice = &self.data.as_ref()[range.clone()];

    if !data_slice.is_empty() {

      let offset = match range.start_bound() {
        Bound::Included(start) => start * size_of::<T>(),
        Bound::Excluded(start) => (start + 1) * size_of::<T>(),
        Bound::Unbounded => 0,
      };

      // may panic if getting larger than size
      gx.write_buffer(&self.buffer, offset as u64, data_slice);
    }
  }
}


// vec helper trait

pub trait CopyExtend<T: Copy> {
  fn copy_extend(&mut self, source: &[T], offset: Option<usize>) -> Range<usize>;
}

impl<T: Copy> CopyExtend<T> for Vec<T> {

  fn copy_extend(&mut self, source: &[T], offset: Option<usize>) -> Range<usize> {

    let start_len = self.len();
    let offset = offset.unwrap_or(start_len);

    match offset.cmp(&start_len) {
      Ordering::Less => {

        let end = offset + source.len();
        let need = isize::try_from(end).unwrap() - isize::try_from(start_len).unwrap();

        // SAFETY: We copy from a type that implements Copy.
        //         Pointers may never overlap because of rust aliasing rules for slices.
        //         We reserve enough space beforehand.
        //         We only set the new length of the vector after copying.
        unsafe {
          if need > 0 { self.reserve(need as usize); }

          copy_nonoverlapping(
            source.as_ptr(),
            self[offset..end].as_mut_ptr(),
            source.len(),
          );

          if need > 0 { self.set_len(end); }
        }

        offset..end
      },
      Ordering::Equal => {
        self.extend_from_slice(source);
        start_len..self.len()
      },
      Ordering::Greater => {
        panic!("offset `{offset}` > self.len() `{start_len}`")
      },
    }
  }
}


// wgpu draw indirect helper traits

pub trait DrawIndirectRanges: Sized {
  fn try_from_ranges(vertex_range: Range<usize>, instance_range: Range<usize>) -> Res<Self>;
  fn vertex_range(&self) -> Res<Range<u32>>;
  fn instance_range(&self) -> Res<Range<u32>>;
}

impl DrawIndirectRanges for DrawIndirectArgs {

  fn try_from_ranges(vertex_range: Range<usize>, instance_range: Range<usize>) -> Res<Self> {
    Ok(Self {
      first_vertex: u32::try_from(vertex_range.start).map_err(|_| "DrawIndirect first_vertex overflow")?,
      vertex_count: u32::try_from(vertex_range.len()).map_err(|_| "DrawIndirect vertex_count overflow")?,
      first_instance: u32::try_from(instance_range.start).map_err(|_| "DrawIndirect first_instance overflow")?,
      instance_count: u32::try_from(instance_range.len()).map_err(|_| "DrawIndirect instance_count overflow")?,
    })
  }

  fn vertex_range(&self) -> Res<Range<u32>> {
    Ok(self.first_vertex..self.first_vertex.checked_add(self.vertex_count).ok_or("DrawIndirect vertex range overflow")?)
  }

  fn instance_range(&self) -> Res<Range<u32>> {
    Ok(self.first_instance..self.first_instance.checked_add(self.instance_count).ok_or("DrawIndirect instance range overflow")?)
  }
}


pub trait DrawIndexedIndirectRanges: Sized {
  fn try_from_offset_ranges(vertex_offset: isize, index_range: Range<usize>, instance_range: Range<usize>) -> Res<Self>;
  fn index_range(&self) -> Res<Range<u32>>;
  fn instance_range(&self) -> Res<Range<u32>>;
}

impl DrawIndexedIndirectRanges for DrawIndexedIndirectArgs {

  fn try_from_offset_ranges(vertex_offset: isize, index_range: Range<usize>, instance_range: Range<usize>) -> Res<Self> {
    Ok(Self {
      base_vertex: i32::try_from(vertex_offset).map_err(|_| "DrawIndexedIndirect base_vertex overflow")?,
      first_index: u32::try_from(index_range.start).map_err(|_| "DrawIndexedIndirect first_index overflow")?,
      index_count: u32::try_from(index_range.len()).map_err(|_| "DrawIndexedIndirect index_count overflow")?,
      first_instance: u32::try_from(instance_range.start).map_err(|_| "DrawIndexedIndirect first_instance overflow")?,
      instance_count: u32::try_from(instance_range.len()).map_err(|_| "DrawIndexedIndirect instance_count overflow")?,
    })
  }

  fn index_range(&self) -> Res<Range<u32>> {
    Ok(self.first_index..self.first_index.checked_add(self.index_count).ok_or("DrawIndexedIndirect index range overflow")?)
  }

  fn instance_range(&self) -> Res<Range<u32>> {
    Ok(self.first_instance..self.first_instance.checked_add(self.instance_count).ok_or("DrawIndexedIndirect instance range overflow")?)
  }
}