// Styled, keyboard-accessible single-select menus. Native selects remain the
// form's source of truth; popup rendering never delegates to an OS menu.
const pickers=new Map();let opened=null;
const svg=(path)=>{const ns='http://www.w3.org/2000/svg',icon=document.createElementNS(ns,'svg'),line=document.createElementNS(ns,'path');icon.setAttribute('viewBox','0 0 20 20');icon.setAttribute('aria-hidden','true');line.setAttribute('d',path);icon.append(line);return icon;};
function labelFor(select){const label=select.labels?.[0];if(label){const clone=label.cloneNode(true);clone.querySelectorAll('select,.picker').forEach(node=>node.remove());const text=clone.textContent.trim();if(text)return text;}return select.getAttribute('aria-label')??select.id;}
function upgrade(select){
  const wrapper=document.createElement('span');wrapper.className='picker';select.before(wrapper);wrapper.append(select);
  const trigger=document.createElement('button');trigger.type='button';trigger.className='picker-trigger';trigger.setAttribute('role','combobox');trigger.setAttribute('aria-haspopup','listbox');trigger.setAttribute('aria-expanded','false');trigger.setAttribute('aria-label',labelFor(select));
  const value=document.createElement('span');value.className='picker-value';trigger.append(value,svg('M5 8l5 5 5-5'));trigger.lastChild.classList.add('picker-chevron');wrapper.append(trigger);
  select.tabIndex=-1;select.setAttribute('aria-hidden','true');
  const list=document.createElement('div');list.className='picker-list';list.id=`cm-menu-${select.id}`;list.setAttribute('role','listbox');list.setAttribute('popover','manual');list.setAttribute('aria-label',trigger.getAttribute('aria-label'));document.body.append(list);trigger.setAttribute('aria-controls',list.id);
  let active=0,rows=[],typed='',typeTimer;
  const current=()=>Math.max(0,select.selectedIndex);
  const enabled=()=>rows.map((_,index)=>index).filter(index=>!select.options[index].disabled);
  function mark(index){active=index;rows.forEach((row,i)=>row.classList.toggle('active',i===active));const row=rows[active];if(row){trigger.setAttribute('aria-activedescendant',row.id);const top=row.offsetTop,bottom=top+row.offsetHeight;if(top<list.scrollTop)list.scrollTop=top-6;else if(bottom>list.scrollTop+list.clientHeight)list.scrollTop=bottom-list.clientHeight+6;}}
  function position(){const rect=trigger.getBoundingClientRect(),width=Math.min(Math.max(rect.width,220),innerWidth-24),below=innerHeight-rect.bottom-10,above=rect.top-10,useAbove=below<Math.min(320,rows.length*41+12)&&above>below,max=Math.max(70,Math.min(320,useAbove?above:below));list.style.width=`${width}px`;list.style.maxHeight=`${max}px`;list.style.left=`${Math.max(12,Math.min(rect.left,innerWidth-width-12))}px`;list.style.top=`${useAbove?Math.max(8,rect.top-list.offsetHeight-7):rect.bottom+7}px`;}
  function close(focus=false){if(opened?.trigger!==trigger)return;if(list.matches(':popover-open'))list.hidePopover();else list.hidden=true;trigger.setAttribute('aria-expanded','false');trigger.removeAttribute('aria-activedescendant');opened=null;if(focus)trigger.focus();}
  function commit(index){if(select.options[index]?.disabled)return;select.selectedIndex=index;select.dispatchEvent(new Event('input',{bubbles:true}));select.dispatchEvent(new Event('change',{bubbles:true}));refresh();close(true);}
  function render(){list.replaceChildren();rows=[...select.options].map((option,index)=>{const row=document.createElement('button');row.type='button';row.className='picker-option';row.id=`${list.id}-${index}`;row.tabIndex=-1;row.setAttribute('role','option');row.setAttribute('aria-selected',String(index===select.selectedIndex));row.disabled=option.disabled;const text=document.createElement('span');text.textContent=option.text;row.append(text,svg('M4 10l4 4 8-8'));row.addEventListener('pointermove',()=>{if(!row.disabled)mark(index);});row.addEventListener('click',()=>commit(index));list.append(row);return row;});}
  function refresh(){value.textContent=select.selectedOptions[0]?.text??'';trigger.disabled=select.disabled;if(opened?.trigger===trigger){render();position();mark(Math.min(active,rows.length-1));}}
  function open(){if(select.disabled)return;opened?.close();render();trigger.setAttribute('aria-expanded','true');if(typeof list.showPopover==='function')list.showPopover();else{list.hidden=false;list.style.display='block';}opened={trigger,list,close,position};position();mark(current());}
  trigger.addEventListener('click',()=>{if(opened?.trigger===trigger)close();else open();});
  trigger.addEventListener('keydown',event=>{
    const isOpen=opened?.trigger===trigger;
    if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){event.preventDefault();event.stopPropagation();if(!isOpen){open();return;}const indexes=enabled(),pos=indexes.indexOf(active);let next=event.key==='Home'?indexes[0]:event.key==='End'?indexes.at(-1):indexes[(pos+(event.key==='ArrowUp'?-1:1)+indexes.length)%indexes.length];if(next!==undefined)mark(next);}
    else if(event.key==='Enter'||event.key===' '){event.preventDefault();event.stopPropagation();if(isOpen)commit(active);else open();}
    else if(event.key==='Escape'&&isOpen){event.preventDefault();event.stopPropagation();close(true);}
    else if(event.key==='Tab'&&isOpen)close();
    else if(event.key.length===1&&!event.ctrlKey&&!event.metaKey&&!event.altKey){event.preventDefault();if(!isOpen)open();clearTimeout(typeTimer);typed+=event.key.toLocaleLowerCase('it');typeTimer=setTimeout(()=>typed='',600);const index=[...select.options].findIndex(option=>!option.disabled&&option.text.toLocaleLowerCase('it').startsWith(typed));if(index>=0)mark(index);}
  });
  select.addEventListener('change',refresh);select.addEventListener('input',refresh);
  new MutationObserver(refresh).observe(select,{childList:true,subtree:true,attributes:true,attributeFilter:['selected','disabled','label']});
  pickers.set(select,{refresh});refresh();
}
export function enhanceSelects(root=document){for(const select of root.querySelectorAll('select'))if(!pickers.has(select))upgrade(select);}
export function refreshSelects(){for(const [select,picker]of pickers)if(select.isConnected)picker.refresh();}
document.addEventListener('pointerdown',event=>{if(opened&&!event.composedPath().includes(opened.trigger)&&!event.composedPath().includes(opened.list))opened.close();},true);
document.addEventListener('focusin',event=>{if(opened&&!event.composedPath().includes(opened.trigger)&&!event.composedPath().includes(opened.list))opened.close();});
document.addEventListener('scroll',()=>opened?.position(),true);
window.addEventListener('resize',()=>opened?.position());
document.addEventListener('visibilitychange',()=>{if(document.hidden)opened?.close();});
