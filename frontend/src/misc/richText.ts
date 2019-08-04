function transformColor(color: string): string {
  const colorAliases = ['red', 'green', 'blue', 'orange', 'yellow', 'pink', 'purple', 'white', 'black', 'gray', 'brown', 'cyan', 'acid'];
  if (colorAliases.includes(color)) {
    return color;
  }

  let components = color.split(',').map(parseFloat);
  if (components.length !== 3 || components.some(component => !(0 <= component && component <= 255))) {
    return null;
  }

  if (components.every(component => component <= 1)) {
    components = components.map(component => component * 255);
  }
  return `rgb(${components.join(', ')})`;
}

// https://wiki.factorio.com/Rich_text
export function transformRichText(text: string): string {
  text = text.trim();

  // [color=rgb]...[/color]
  text = text.replace(
      /\[color=([^\]]*)\](.*)\[\/color\]/g,
      (match, color, text) => {
        color = transformColor(color);
        return color === null ? text : `<span style="color: ${color}">${text}</span>`;
      });

  // [font=font-name]...[/font]
  text = text.replace(/\[font=([^\]]*)\](.*)\[\/font\]/g, '$2');

  // [img=class/name] and [img=class.name]
  text = text.replace(/\[img=(.*)[/.](.*)\]/g, '[$1=$2]');
  // [item=name] and others
  text = text.replace(
      /\[(?:item|entity|technology|recipe|item-group|fluid|tile|virtual-signal|achievement|equipment)=([^\]]*)\]/g,
      '<img height="24" width="24" src="/icons/$1.png">',
  );

  return text;
}
